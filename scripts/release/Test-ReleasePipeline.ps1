$ErrorActionPreference = "Stop"
Import-Module (Join-Path $PSScriptRoot "ReleasePipeline.psm1") -Force

$script:Passed = 0
$script:Failed = 0

function Test-Case {
    param([string] $Name, [scriptblock] $Body)
    try {
        & $Body
        $script:Passed++
        Write-Host "PASS $Name"
    }
    catch {
        $script:Failed++
        Write-Host "FAIL $Name`: $($_.Exception.Message)"
    }
}

function Assert-Equal($Actual, $Expected) {
    if ($Actual -ne $Expected) {
        throw "Expected '$Expected', got '$Actual'."
    }
}

function Assert-Throws([scriptblock] $Body) {
    try { & $Body } catch { return }
    throw "Expected the operation to fail."
}

Test-Case "strict stable SemVer syntax" {
    Assert-Equal (ConvertTo-StableSemVer "1.10.0").Tag "v1.10.0"
    foreach ($invalid in @("1.2", "v1.2.3", "01.2.3", "1.02.3", "1.2.03", "1.2.3-beta", "1.2.3+build", "-1.2.3")) {
        Assert-Throws { ConvertTo-StableSemVer $invalid }
    }
}

Test-Case "numeric precedence" {
    Assert-Equal (Compare-StableSemVer "1.9.0" "1.10.0") -1
    Assert-Equal (Compare-StableSemVer "2.0.0" "1.99.99") 1
    Assert-Equal (Compare-StableSemVer "1.2.3" "1.2.3") 0
}

Test-Case "five declaration agreement" {
    $root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
    $versions = Get-VersionDeclarations -Root $root
    Assert-Equal $versions.Count 5
    $expected = [string]$versions["src-tauri/tauri.conf.json"]
    Assert-Equal (Assert-VersionAgreement -Declarations $versions) $expected
    $versions["package.json"] = if ($expected -eq "0.0.0") { "0.0.1" } else { "0.0.0" }
    Assert-Throws { Assert-VersionAgreement -Declarations $versions }
}

Test-Case "greatest stable release filtering" {
    $releases = @(
        [pscustomobject]@{ tag_name = "release-abc"; draft = $false; prerelease = $false; target_commitish = "a" },
        [pscustomobject]@{ tag_name = "v1.9.0"; draft = $false; prerelease = $false; target_commitish = "b" },
        [pscustomobject]@{ tag_name = "v1.10.0"; draft = $false; prerelease = $false; target_commitish = "c" },
        [pscustomobject]@{ tag_name = "v9.0.0"; draft = $true; prerelease = $false; target_commitish = "d" },
        [pscustomobject]@{ tag_name = "v8.0.0"; draft = $false; prerelease = $true; target_commitish = "e" }
    )
    Assert-Equal (Get-GreatestStableRelease -Releases $releases).tag_name "v1.10.0"
    Assert-Throws { Assert-ReleaseVersion -Version "1.10.0" -Releases $releases -MigrationFloor "0.1.0" }
    Assert-Equal (Assert-ReleaseVersion -Version "1.11.0" -Releases $releases -MigrationFloor "0.1.0") "v1.11.0"
}

Test-Case "GitHub pagination works without incompatible jq flags" {
    $pages = '[[{"tag_name":"v1.9.0"}],[{"tag_name":"v1.10.0"}]]'
    $releases = @(ConvertFrom-GitHubApiPages $pages)
    Assert-Equal $releases.Count 2
    Assert-Equal $releases[1].tag_name "v1.10.0"
    Assert-Equal @(ConvertFrom-GitHubApiPages '[[]]').Count 0

    $workflow = Get-Content (Join-Path $PSScriptRoot "..\..\.github\workflows\release.yml") -Raw
    if ($workflow -match '--slurp[^\r\n]*--jq|--jq[^\r\n]*--slurp') {
        throw "GitHub CLI does not support combining --slurp with --jq."
    }
}

Test-Case "quality runs only for non-release pull requests" {
    $workflow = Get-Content (Join-Path $PSScriptRoot "..\..\.github\workflows\quality.yml") -Raw
    if ($workflow -match '(?m)^\s{2}push:') {
        throw "Quality must not run for branch pushes."
    }
    foreach ($required in @(
        'pull_request:',
        "github.head_ref != 'release'",
        "!startsWith(github.head_ref, 'release/')"
    )) {
        if (-not $workflow.Contains($required)) {
            throw "Quality workflow is missing release-branch exclusion '$required'."
        }
    }
}

Test-Case "draft releases are safely replaced and verified by immutable release id" {
    $workflow = Get-Content (Join-Path $PSScriptRoot "..\..\.github\workflows\release.yml") -Raw
    if ($workflow.Contains('gh api "/repos/$env:TRUSTED_REPOSITORY/releases/tags/$env:RELEASE_TAG"')) {
        throw "Draft releases cannot be read reliably through the release-by-tag endpoint."
    }
    if ($workflow.Contains('$draftMatches') -or $workflow.Contains('Could not refresh draft release state.')) {
        throw "A newly created draft must not be rediscovered through an eventually consistent release listing."
    }
    if ($workflow.Contains('gh release upload $env:RELEASE_TAG') -or $workflow.Contains('gh release edit $env:RELEASE_TAG')) {
        throw "Draft upload and publish mutations must use the immutable release ID."
    }
    foreach ($required in @(
        '$ownedDraftName = "^periScope $escapedVersion \(PR #[1-9][0-9]*, [0-9a-f]{7}\)$"',
        '$draft = gh api --method PATCH "/repos/$env:TRUSTED_REPOSITORY/releases/$($existing[0].id)" --input $draftRequestPath | ConvertFrom-Json',
        'gh api --method DELETE "/repos/$env:TRUSTED_REPOSITORY/releases/assets/$($staleAsset.id)"',
        'if ($LASTEXITCODE -ne 0 -or @($draft.assets).Count -ne 0) { throw "Could not establish an empty retry draft." }',
        '$draft = gh api --method POST "/repos/$env:TRUSTED_REPOSITORY/releases" --input $draftRequestPath | ConvertFrom-Json',
        'if ([long]$draft.id -le 0 -or -not $draft.draft -or $draft.tag_name -cne $env:RELEASE_TAG -or $draft.target_commitish -cne $sha)',
        '$uploadBase = "https://uploads.github.com/repos/$env:TRUSTED_REPOSITORY/releases/$($draft.id)/assets"',
        'Invoke-RestMethod -Method Post -Uri $uploadUri',
        '$verified = gh api "/repos/$env:TRUSTED_REPOSITORY/releases/$($draft.id)"',
        '$published = gh api --method PATCH "/repos/$env:TRUSTED_REPOSITORY/releases/$($draft.id)" --input $publishRequestPath | ConvertFrom-Json'
    )) {
        if (-not $workflow.Contains($required)) {
            throw "Release workflow is missing the required ID-based lookup: $required"
        }
    }
}

Test-Case "retry and conflict inspection" {
    $sha = "0123456789abcdef0123456789abcdef01234567"
    $matching = [pscustomobject]@{ tag_name = "v0.2.0"; draft = $true; prerelease = $false; target_commitish = $sha; assets = @() }
    Assert-Equal (Get-ReleaseDisposition -Version "0.2.0" -MergeSha $sha -Releases @($matching)).State "replace-draft"
    $matching.target_commitish = ("f" * 40)
    Assert-Equal (Get-ReleaseDisposition -Version "0.2.0" -MergeSha $sha -Releases @($matching)).State "replace-draft"
    $matching.draft = $false
    Assert-Throws { Get-ReleaseDisposition -Version "0.2.0" -MergeSha $sha -Releases @($matching) }
    $matching.target_commitish = $sha
    Assert-Equal (Get-ReleaseDisposition -Version "0.2.0" -MergeSha $sha -Releases @($matching)).State "verify-published"
}

Test-Case "deterministic handoff metadata and public manifest" {
    $temp = Join-Path ([System.IO.Path]::GetTempPath()) "periscope-release-tests-$PID"
    New-Item -ItemType Directory -Path $temp -Force | Out-Null
    try {
        $installer = Join-Path $temp "periScope_0.2.0_x64-setup.exe"
        $signature = "$installer.sig"
        [System.IO.File]::WriteAllBytes($installer, [byte[]](1, 2, 3, 4))
        Set-Content -LiteralPath $signature -Value "signed-value" -NoNewline
        $sha = "0123456789abcdef0123456789abcdef01234567"
        $handoff = New-ReleaseHandoff -Repository "msugenius/periscope" -MergeSha $sha -Version "0.2.0" -InstallerPath $installer -SignaturePath $signature
        Assert-Equal $handoff.schemaVersion "2"
        Assert-Equal (Test-ReleaseHandoff -Handoff $handoff -Root $temp -Repository "msugenius/periscope" -MergeSha $sha -Version "0.2.0") $true

        $metadata = New-UpdaterMetadata -Repository "msugenius/periscope" -Version "0.2.0" -MergeSha $sha -Notes "Notes" -InstallerName (Split-Path $installer -Leaf) -Signature "signed-value"
        Assert-Equal $metadata.platforms."windows-x86_64".signature "signed-value"
        if ($metadata.platforms."windows-x86_64".url -notlike "*/download/v0.2.0/*") { throw "Updater URL is not immutable." }

        $metadataPath = Join-Path $temp "latest.json"
        $metadata | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $metadataPath
        $manifest = New-PublicReleaseManifest -Repository "msugenius/periscope" -MergeSha $sha -Version "0.2.0" -InstallerPath $installer -SignaturePath $signature -UpdaterMetadataPath $metadataPath
        Assert-Equal (Test-PublicReleaseManifest -Manifest $manifest -Root $temp -Repository "msugenius/periscope" -MergeSha $sha -Version "0.2.0") $true
        Assert-Equal ($manifest.PSObject.Properties.Name -contains "publishedAt") $false
    }
    finally {
        Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "$script:Passed passed; $script:Failed failed"
if ($script:Failed -gt 0) { exit 1 }
