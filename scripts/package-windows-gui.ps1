#requires -Version 5.1
<#
.SYNOPSIS
  Full Windows GUI bundle: release CLI/service -> Tauri resources -> NSIS + MSI installers.

.DESCRIPTION
  1. cargo build --release for agent-diva-cli and agent-diva-service
  2. pnpm bundle:prepare (stages binaries into src-tauri/resources)
  3. Optional: prefetch NSIS toolchain into %LOCALAPPDATA%\tauri\NSIS (avoids Tauri downloader timeouts)
  4. pnpm tauri build

  Run from repo root or any directory:
    pwsh -File scripts/package-windows-gui.ps1

.PARAMETER SkipCargo
  Skip Rust release build (use when target/release binaries are already fresh).

.PARAMETER SkipPrepare
  Skip pnpm bundle:prepare.

.PARAMETER SkipNsisPrecache
  Do not download/extract NSIS into the Tauri cache when missing.

.PARAMETER SkipPnpmInstall
  Do not run `pnpm install` when node_modules is absent.
#>
param(
    [switch] $SkipCargo,
    [switch] $SkipPrepare,
    [switch] $SkipNsisPrecache,
    [switch] $SkipPnpmInstall
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$GuiRoot = Join-Path $RepoRoot "agent-diva-gui"
$NsisUrl = "https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip"
$NsisSha1Expected = "EF7FF767E5CBD9EDD22ADD3A32C9B8F4500BB10D"
$UtilsUrl = "https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll"
$UtilsSha1Expected = "75197FEE3C6A814FE035788D1C34EAD39349B860"

function Write-Step([string] $Message) {
    Write-Host ""
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Assert-Command([string] $Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command not found on PATH: $Name"
    }
}

function Test-NsisToolchainReady {
    $base = Join-Path $env:LOCALAPPDATA "tauri\NSIS"
    $candidates = @(
        (Join-Path $base "makensis.exe"),
        (Join-Path $base "Bin\makensis.exe")
    )
    foreach ($p in $candidates) {
        if (Test-Path -LiteralPath $p) {
            $plugin = Join-Path $base "Plugins\x86-unicode\additional\nsis_tauri_utils.dll"
            return (Test-Path -LiteralPath $plugin)
        }
    }
    return $false
}

function Get-Sha1Upper([string] $Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA1).Hash
}

function Ensure-NsisPrecache {
    if (Test-NsisToolchainReady) {
        Write-Host "NSIS toolchain already present under $($env:LOCALAPPDATA)\tauri\NSIS"
        return
    }

    Write-Step "Prefetching NSIS toolchain (Tauri bundler cache)"
    Assert-Command curl.exe

    $tauriCache = Join-Path $env:LOCALAPPDATA "tauri"
    New-Item -ItemType Directory -Force -Path $tauriCache | Out-Null

    $zip = Join-Path $env:TEMP "nsis-3.11-agent-diva.zip"
    curl.exe -fsSL --connect-timeout 60 --max-time 900 -o $zip $NsisUrl

    $sha = Get-Sha1Upper $zip
    if ($sha -ne $NsisSha1Expected) {
        throw "NSIS zip SHA1 mismatch: got $sha expected $NsisSha1Expected"
    }

    $extracted = Join-Path $tauriCache "nsis-3.11"
    if (Test-Path -LiteralPath $extracted) {
        Remove-Item -LiteralPath $extracted -Recurse -Force
    }
    Expand-Archive -LiteralPath $zip -DestinationPath $tauriCache -Force

    $dest = Join-Path $tauriCache "NSIS"
    if (Test-Path -LiteralPath $dest) {
        Remove-Item -LiteralPath $dest -Recurse -Force
    }
    Rename-Item -LiteralPath $extracted -NewName "NSIS"

    $pluginDir = Join-Path $dest "Plugins\x86-unicode\additional"
    New-Item -ItemType Directory -Force -Path $pluginDir | Out-Null
    $dllPath = Join-Path $pluginDir "nsis_tauri_utils.dll"
    curl.exe -fsSL --max-time 300 -o $dllPath $UtilsUrl
    $dllSha = Get-Sha1Upper $dllPath
    if ($dllSha -ne $UtilsSha1Expected) {
        throw "nsis_tauri_utils.dll SHA1 mismatch: got $dllSha expected $UtilsSha1Expected"
    }

    Write-Host "NSIS ready at $dest"
}

Write-Host "Repository root: $RepoRoot"

Assert-Command cargo
Assert-Command pnpm
Assert-Command python

if (-not (Test-Path -LiteralPath $GuiRoot)) {
    throw "GUI directory not found: $GuiRoot"
}

Push-Location $RepoRoot
try {
    if (-not $SkipCargo) {
        Write-Step "cargo build --release (agent-diva-cli, agent-diva-service)"
        cargo build --release -p agent-diva-cli -p agent-diva-service
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
    }

    Push-Location $GuiRoot
    try {
        if (-not $SkipPnpmInstall -and -not (Test-Path -LiteralPath (Join-Path $GuiRoot "node_modules"))) {
            Write-Step "pnpm install (node_modules missing)"
            pnpm install
            if ($LASTEXITCODE -ne 0) { throw "pnpm install failed with exit code $LASTEXITCODE" }
        }

        if (-not $SkipPrepare) {
            Write-Step "pnpm bundle:prepare"
            pnpm run bundle:prepare
            if ($LASTEXITCODE -ne 0) { throw "bundle:prepare failed with exit code $LASTEXITCODE" }
        }

        if (-not $SkipNsisPrecache) {
            Ensure-NsisPrecache
        }

        Write-Step "pnpm tauri build"
        pnpm tauri build
        if ($LASTEXITCODE -ne 0) { throw "tauri build failed with exit code $LASTEXITCODE" }
    }
    finally {
        Pop-Location
    }

    $bundleRoot = Join-Path $RepoRoot "target\release\bundle"
    Write-Step "Done"
    Write-Host "Installers (if present):"
    if (Test-Path -LiteralPath (Join-Path $bundleRoot "nsis")) {
        Get-ChildItem (Join-Path $bundleRoot "nsis") -Filter "*.exe" -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "  $($_.FullName)" }
    }
    if (Test-Path -LiteralPath (Join-Path $bundleRoot "msi")) {
        Get-ChildItem (Join-Path $bundleRoot "msi") -Filter "*.msi" -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "  $($_.FullName)" }
    }
    Write-Host "GUI binary:"
    Write-Host "  $(Join-Path $RepoRoot 'target\release\agent-diva-gui.exe')"
}
finally {
    Pop-Location
}
