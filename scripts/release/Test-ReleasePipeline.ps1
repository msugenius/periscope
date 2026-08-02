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
    Assert-Equal (Assert-VersionAgreement -Declarations $versions) "0.2.0"
    $versions["package.json"] = "0.3.0"
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

Test-Case "draft releases are verified by immutable release id" {
    $workflow = Get-Content (Join-Path $PSScriptRoot "..\..\.github\workflows\release.yml") -Raw
    if ($workflow.Contains('gh api "/repos/$env:TRUSTED_REPOSITORY/releases/tags/$env:RELEASE_TAG"')) {
        throw "Draft releases cannot be read reliably through the release-by-tag endpoint."
    }

    foreach ($required in @(
        '$draft = gh api "/repos/$env:TRUSTED_REPOSITORY/releases/$($draftMatches[0].id)"',
        '$verified = gh api "/repos/$env:TRUSTED_REPOSITORY/releases/$($draft.id)"',
        '$published = gh api "/repos/$env:TRUSTED_REPOSITORY/releases/$($draft.id)"'
    )) {
        if (-not $workflow.Contains($required)) {
            throw "Release workflow is missing the required ID-based lookup: $required"
        }
    }
}

Test-Case "retry and conflict inspection" {
    $sha = "0123456789abcdef0123456789abcdef01234567"
    $matching = [pscustomobject]@{ tag_name = "v0.2.0"; draft = $true; prerelease = $false; target_commitish = $sha; assets = @() }
    Assert-Equal (Get-ReleaseDisposition -Version "0.2.0" -MergeSha $sha -Releases @($matching)).State "resume-draft"
    $matching.draft = $false
    Assert-Equal (Get-ReleaseDisposition -Version "0.2.0" -MergeSha $sha -Releases @($matching)).State "verify-published"
    $matching.target_commitish = ("f" * 40)
    Assert-Throws { Get-ReleaseDisposition -Version "0.2.0" -MergeSha $sha -Releases @($matching) }
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
