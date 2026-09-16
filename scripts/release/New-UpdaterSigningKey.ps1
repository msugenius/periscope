[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateNotNullOrEmpty()]
    [string] $OutputDirectory
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# Keep the original PowerShell entry point, but use the same verified generator
# as npm run signing:generate. Tauri's .pub output is already Base64 encoded.
$generator = Join-Path $PSScriptRoot "signing.mjs"
& node $generator generate --output-dir $OutputDirectory
if ($LASTEXITCODE -ne 0) {
    throw "Updater signing setup failed. See the error above."
}
