# Requires PowerShell 7+, Rust stable MSVC, Visual Studio Build Tools + SDK.
# Run from any directory: pwsh -File tools/verify-windows.ps1
[CmdletBinding()]
param(
    [switch]$SkipTests
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$project = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $project
try {
    $cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargoCommand) {
        throw "cargo not found. Install Rust stable MSVC toolchain and Visual Studio C++ Build Tools."
    }

    function Invoke-CargoStep {
        param([string[]]$CargoArgs)
        Write-Host "Running: cargo $($CargoArgs -join ' ')" -ForegroundColor Cyan
        & cargo @CargoArgs
        if ($LASTEXITCODE -ne 0) {
            throw "cargo $($CargoArgs -join ' ') failed with code $LASTEXITCODE"
        }
    }

    & cargo --version
    if ($LASTEXITCODE -ne 0) { throw "cargo --version failed" }

    Invoke-CargoStep -CargoArgs @("check")
    if (-not $SkipTests) {
        Invoke-CargoStep -CargoArgs @("test")
    }
    Invoke-CargoStep -CargoArgs @("build", "--release")

    $binary = Join-Path $project "target/release/zillaplayer.exe"
    if (-not (Test-Path $binary)) {
        throw "Windows executable was not found at $binary"
    }
    Write-Host "Build SUCCESS. Executable: $binary" -ForegroundColor Green
    $lock = Join-Path $project "Cargo.lock"
    if (Test-Path $lock) {
        Write-Host "Cargo.lock generated. Commit it after reviewing dependency resolution."
    } else {
        Write-Warning "Cargo.lock missing: dependency versions are not fully locked."
    }
    Write-Warning "This script cannot verify audible playback or hardware-specific latency. Run the manual smoke tests."
} finally {
    Pop-Location
}
