param(
    [string]$BinaryPath,
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if (-not $BinaryPath) {
    $BinaryPath = Join-Path $root 'target/release/leocard.exe'
}
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $root 'artifacts/texas-rebuy-windows-x86_64'
}
if (-not (Test-Path -LiteralPath $BinaryPath -PathType Leaf)) {
    throw "Client executable not found: $BinaryPath"
}

New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$name = 'leocard-texas-rebuy-windows-x86_64.exe'
$executable = Join-Path $OutputDirectory $name
Copy-Item -LiteralPath $BinaryPath -Destination $executable -Force
Copy-Item -LiteralPath (Join-Path $root 'LICENSE') -Destination (Join-Path $OutputDirectory 'LICENSE.txt') -Force
Copy-Item -LiteralPath (Join-Path $root 'assets/ASSETS.md') -Destination (Join-Path $OutputDirectory 'ASSETS.md') -Force

# Each launcher changes only its child process environment, never the real profile.
foreach ($account in 'A', 'B', 'C') {
    $launcher = @"
@echo off
setlocal
set "LEOCARD_CONFIG_DIR=%LOCALAPPDATA%\LeoCard-Rebuy-Test\$account"
start "" /D "%~dp0" "%~dp0$name"
endlocal
"@
    Set-Content -LiteralPath (Join-Path $OutputDirectory "start-test-account-$account.cmd") -Value $launcher -Encoding Ascii
}

$hash = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -LiteralPath "$executable.sha256" -Value "$hash  $name" -Encoding Ascii
Write-Output "Packaged Windows test client: $OutputDirectory"
