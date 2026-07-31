Set-StrictMode -Version Latest

function ConvertTo-StableSemVer {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string] $Version)

    if ($Version -notmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') {
        throw "Version '$Version' is not strict stable MAJOR.MINOR.PATCH SemVer."
    }
    try {
        $major = [uint64]::Parse($Matches[1])
        $minor = [uint64]::Parse($Matches[2])
        $patch = [uint64]::Parse($Matches[3])
    }
    catch {
        throw "Version '$Version' contains an identifier outside the supported numeric range."
    }
    [pscustomobject][ordered]@{
        Text = $Version
        Tag = "v$Version"
        Major = $major
        Minor = $minor
        Patch = $patch
    }
}

function Compare-StableSemVer {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string] $Left,
        [Parameter(Mandatory)][string] $Right
    )
    $a = ConvertTo-StableSemVer $Left
    $b = ConvertTo-StableSemVer $Right
    foreach ($field in @('Major', 'Minor', 'Patch')) {
        if ($a.$field -lt $b.$field) { return -1 }
        if ($a.$field -gt $b.$field) { return 1 }
    }
    return 0
}

function Get-VersionDeclarations {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string] $Root)

    $rootPath = (Resolve-Path -LiteralPath $Root).Path
    $tauri = Get-Content -LiteralPath (Join-Path $rootPath 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
    $cargoText = Get-Content -LiteralPath (Join-Path $rootPath 'src-tauri/Cargo.toml') -Raw
    if ($cargoText -notmatch '(?ms)^\[package\].*?^version\s*=\s*"([^"]+)"') {
        throw 'Could not read the package version from src-tauri/Cargo.toml.'
    }
    $cargoVersion = $Matches[1]
    $lockText = Get-Content -LiteralPath (Join-Path $rootPath 'src-tauri/Cargo.lock') -Raw
    if ($lockText -notmatch '(?ms)^\[\[package\]\]\s*^name\s*=\s*"periscope"\s*^version\s*=\s*"([^"]+)"') {
        throw 'Could not read the periScope version from src-tauri/Cargo.lock.'
    }
    $cargoLockVersion = $Matches[1]
    $package = Get-Content -LiteralPath (Join-Path $rootPath 'package.json') -Raw | ConvertFrom-Json
    $packageLockText = Get-Content -LiteralPath (Join-Path $rootPath 'package-lock.json') -Raw
    if ($packageLockText -notmatch '(?s)"packages"\s*:\s*\{\s*""\s*:\s*\{.*?"version"\s*:\s*"([^"]+)"') {
        throw 'Could not read the root version from package-lock.json.'
    }
    $packageLockVersion = $Matches[1]
    [ordered]@{
        'src-tauri/tauri.conf.json' = [string]$tauri.version
        'src-tauri/Cargo.toml' = [string]$cargoVersion
        'src-tauri/Cargo.lock' = [string]$cargoLockVersion
        'package.json' = [string]$package.version
        'package-lock.json' = [string]$packageLockVersion
    }
}

function Assert-VersionAgreement {
    [CmdletBinding()]
    param([Parameter(Mandatory)][System.Collections.IDictionary] $Declarations)

    if ($Declarations.Count -ne 5) {
        throw "Expected five version declarations; found $($Declarations.Count)."
    }
    $authority = [string]$Declarations['src-tauri/tauri.conf.json']
    ConvertTo-StableSemVer $authority | Out-Null
    foreach ($entry in $Declarations.GetEnumerator()) {
        ConvertTo-StableSemVer ([string]$entry.Value) | Out-Null
        if ([string]$entry.Value -cne $authority) {
            throw "Version mismatch: $($entry.Key) declares '$($entry.Value)' instead of '$authority'."
        }
    }
    return $authority
}

function Get-StableReleases {
    [CmdletBinding()]
    param([Parameter(Mandatory)][AllowEmptyCollection()][object[]] $Releases)

    @($Releases | Where-Object {
        -not $_.draft -and -not $_.prerelease -and
        [string]$_.tag_name -match '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$'
    })
}

function Get-GreatestStableRelease {
    [CmdletBinding()]
    param([Parameter(Mandatory)][AllowEmptyCollection()][object[]] $Releases)

    $greatest = $null
    foreach ($release in @(Get-StableReleases -Releases $Releases)) {
        if (-not $greatest -or (Compare-StableSemVer $release.tag_name.Substring(1) $greatest.tag_name.Substring(1)) -gt 0) {
            $greatest = $release
        }
    }
    return $greatest
}

function Assert-ReleaseVersion {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string] $Version,
        [Parameter(Mandatory)][AllowEmptyCollection()][object[]] $Releases,
        [string] $MigrationFloor = '0.1.0'
    )
    $parsed = ConvertTo-StableSemVer $Version
    ConvertTo-StableSemVer $MigrationFloor | Out-Null
    if ((Compare-StableSemVer $Version $MigrationFloor) -le 0) {
        throw "Release version $Version must exceed migration floor $MigrationFloor."
    }
    $greatest = Get-GreatestStableRelease -Releases $Releases
    if ($greatest -and (Compare-StableSemVer $Version $greatest.tag_name.Substring(1)) -le 0) {
        throw "Release version $Version must exceed greatest stable release $($greatest.tag_name)."
    }
    return $parsed.Tag
}

function Get-ReleaseDisposition {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string] $Version,
        [Parameter(Mandatory)][string] $MergeSha,
        [Parameter(Mandatory)][AllowEmptyCollection()][object[]] $Releases
    )
    $tag = (ConvertTo-StableSemVer $Version).Tag
    if ($MergeSha -cnotmatch '^[0-9a-f]{40}$') { throw 'Merge SHA must be 40 lowercase hexadecimal characters.' }
    $matching = @($Releases | Where-Object tag_name -eq $tag)
    if ($matching.Count -gt 1) { throw "Multiple releases use tag $tag." }
    if ($matching.Count -eq 0) {
        return [pscustomobject][ordered]@{ State = 'build-new'; Tag = $tag; Release = $null }
    }
    $release = $matching[0]
    if ([string]$release.target_commitish -cne $MergeSha) {
        throw "Tag $tag targets another commit."
    }
    $state = if ($release.draft) { 'resume-draft' } else { 'verify-published' }
    [pscustomobject][ordered]@{ State = $state; Tag = $tag; Release = $release }
}

function Get-FileEvidence {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string] $Path)
    $file = Get-Item -LiteralPath $Path
    if (-not $file.PSIsContainer -and $file.Length -gt 0) {
        return [pscustomobject][ordered]@{
            name = $file.Name
            sizeBytes = [long]$file.Length
            sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    }
    throw "Release file '$Path' is missing, empty, or not a file."
}

function New-ReleaseHandoff {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string] $Repository,
        [Parameter(Mandatory)][string] $MergeSha,
        [Parameter(Mandatory)][string] $Version,
        [Parameter(Mandatory)][string] $InstallerPath,
        [Parameter(Mandatory)][string] $SignaturePath
    )
    $tag = (ConvertTo-StableSemVer $Version).Tag
    if ($MergeSha -cnotmatch '^[0-9a-f]{40}$') { throw 'Merge SHA must be 40 lowercase hexadecimal characters.' }
    [pscustomobject][ordered]@{
        schemaVersion = '2'
        repository = $Repository
        mergeSha = $MergeSha
        version = $Version
        tag = $tag
        installer = Get-FileEvidence $InstallerPath
        signature = Get-FileEvidence $SignaturePath
    }
}

function Test-FileEvidence {
    param([object] $Evidence, [string] $Root, [string] $Pattern)
    if ([string]$Evidence.name -ne [System.IO.Path]::GetFileName([string]$Evidence.name) -or [string]$Evidence.name -cnotmatch $Pattern) { return $false }
    $path = Join-Path $Root ([string]$Evidence.name)
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return $false }
    $actual = Get-FileEvidence $path
    $actual.sizeBytes -eq [long]$Evidence.sizeBytes -and $actual.sha256 -ceq [string]$Evidence.sha256
}

function Test-ReleaseHandoff {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][object] $Handoff,
        [Parameter(Mandatory)][string] $Root,
        [Parameter(Mandatory)][string] $Repository,
        [Parameter(Mandatory)][string] $MergeSha,
        [Parameter(Mandatory)][string] $Version
    )
    $escaped = [regex]::Escape($Version)
    $Handoff.schemaVersion -ceq '2' -and
        $Handoff.repository -ceq $Repository -and
        $Handoff.mergeSha -ceq $MergeSha -and
        $Handoff.version -ceq $Version -and
        $Handoff.tag -ceq "v$Version" -and
        (Test-FileEvidence $Handoff.installer $Root "^periScope_${escaped}_x64-setup\.exe$") -and
        (Test-FileEvidence $Handoff.signature $Root "^periScope_${escaped}_x64-setup\.exe\.sig$")
}

function New-UpdaterMetadata {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string] $Repository,
        [Parameter(Mandatory)][string] $Version,
        [Parameter(Mandatory)][string] $MergeSha,
        [Parameter(Mandatory)][string] $Notes,
        [Parameter(Mandatory)][string] $InstallerName,
        [Parameter(Mandatory)][string] $Signature
    )
    $tag = (ConvertTo-StableSemVer $Version).Tag
    if ($MergeSha -cnotmatch '^[0-9a-f]{40}$') { throw 'Merge SHA must be 40 lowercase hexadecimal characters.' }
    if ([string]::IsNullOrWhiteSpace($Notes) -or [string]::IsNullOrWhiteSpace($Signature)) { throw 'Updater notes and signature must be non-empty.' }
    if ($InstallerName -ne [System.IO.Path]::GetFileName($InstallerName)) { throw 'Installer name must be a safe base name.' }
    [pscustomobject][ordered]@{
        version = $Version
        notes = $Notes
        sourceCommit = $MergeSha
        platforms = [pscustomobject][ordered]@{
            'windows-x86_64' = [pscustomobject][ordered]@{
                url = "https://github.com/$Repository/releases/download/$tag/$InstallerName"
                signature = $Signature
            }
        }
    }
}

function New-PublicReleaseManifest {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string] $Repository,
        [Parameter(Mandatory)][string] $MergeSha,
        [Parameter(Mandatory)][string] $Version,
        [Parameter(Mandatory)][string] $InstallerPath,
        [Parameter(Mandatory)][string] $SignaturePath,
        [Parameter(Mandatory)][string] $UpdaterMetadataPath
    )
    [pscustomobject][ordered]@{
        schemaVersion = '2'
        repository = $Repository
        mergeSha = $MergeSha
        version = $Version
        tag = (ConvertTo-StableSemVer $Version).Tag
        installer = Get-FileEvidence $InstallerPath
        signature = Get-FileEvidence $SignaturePath
        updaterMetadata = Get-FileEvidence $UpdaterMetadataPath
    }
}

function Test-PublicReleaseManifest {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][object] $Manifest,
        [Parameter(Mandatory)][string] $Root,
        [Parameter(Mandatory)][string] $Repository,
        [Parameter(Mandatory)][string] $MergeSha,
        [Parameter(Mandatory)][string] $Version
    )
    $escaped = [regex]::Escape($Version)
    $Manifest.schemaVersion -ceq '2' -and
        $Manifest.repository -ceq $Repository -and
        $Manifest.mergeSha -ceq $MergeSha -and
        $Manifest.version -ceq $Version -and
        $Manifest.tag -ceq "v$Version" -and
        (Test-FileEvidence $Manifest.installer $Root "^periScope_${escaped}_x64-setup\.exe$") -and
        (Test-FileEvidence $Manifest.signature $Root "^periScope_${escaped}_x64-setup\.exe\.sig$") -and
        (Test-FileEvidence $Manifest.updaterMetadata $Root '^latest\.json$')
}

Export-ModuleMember -Function @(
    'ConvertTo-StableSemVer', 'Compare-StableSemVer', 'Get-VersionDeclarations',
    'Assert-VersionAgreement', 'Get-StableReleases', 'Get-GreatestStableRelease',
    'Assert-ReleaseVersion', 'Get-ReleaseDisposition', 'Get-FileEvidence',
    'New-ReleaseHandoff', 'Test-ReleaseHandoff', 'New-UpdaterMetadata',
    'New-PublicReleaseManifest', 'Test-PublicReleaseManifest'
)
