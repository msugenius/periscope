[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateNotNullOrEmpty()]
    [string] $OutputDirectory
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$tauriCommand = Join-Path $repositoryRoot "node_modules\.bin\tauri.cmd"
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
$privateKeyPath = Join-Path $outputRoot "updater.key"
$publicKeyPath = "$privateKeyPath.pub"
$passwordPath = "$privateKeyPath.password"
$tauriPublicKeyPath = Join-Path $outputRoot "tauri-pubkey.txt"
$outputs = @(
    $privateKeyPath,
    $publicKeyPath,
    $passwordPath,
    $tauriPublicKeyPath
)

foreach ($path in $outputs) {
    if (Test-Path -LiteralPath $path) {
        throw "Refusing to overwrite existing key material: $path"
    }
}

if (-not (Test-Path -LiteralPath $tauriCommand -PathType Leaf)) {
    throw "Tauri CLI is not installed. Run 'npm ci' before generating keys."
}

New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null

$passwordBytes = New-Object byte[] 32
$random = [System.Security.Cryptography.RandomNumberGenerator]::Create()
try {
    $random.GetBytes($passwordBytes)
}
finally {
    $random.Dispose()
}
$password = ([System.BitConverter]::ToString($passwordBytes)).Replace("-", "").ToLowerInvariant()
[Array]::Clear($passwordBytes, 0, $passwordBytes.Length)

Push-Location $repositoryRoot
try {
    & $tauriCommand signer generate --ci --write-keys $privateKeyPath --password $password
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri signer failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $privateKeyPath -PathType Leaf) -or
    -not (Test-Path -LiteralPath $publicKeyPath -PathType Leaf)) {
    throw "Tauri signer did not produce the expected private and public key files."
}

$utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($passwordPath, $password, $utf8WithoutBom)

$publicKey = [System.IO.File]::ReadAllText($publicKeyPath)
if ([string]::IsNullOrWhiteSpace($publicKey)) {
    throw "Generated public key is empty."
}
$tauriPublicKey = [System.Convert]::ToBase64String(
    [System.Text.Encoding]::UTF8.GetBytes($publicKey)
)
[System.IO.File]::WriteAllText($tauriPublicKeyPath, $tauriPublicKey, $utf8WithoutBom)

Write-Host "Updater signing material generated successfully."
Write-Host ""
Write-Host "Private key (GitHub secret TAURI_SIGNING_PRIVATE_KEY):"
Write-Host "  $privateKeyPath"
Write-Host "Password (GitHub secret TAURI_SIGNING_PRIVATE_KEY_PASSWORD):"
Write-Host "  $passwordPath"
Write-Host "Public key source:"
Write-Host "  $publicKeyPath"
Write-Host "Base64 public key (plugins.updater.pubkey in tauri.conf.json):"
Write-Host "  $tauriPublicKeyPath"
Write-Host ""
Write-Warning "Keep the private key and password outside source control and back them up securely."
Write-Warning "Rotating this public key requires existing users to install the first newly signed release manually."
