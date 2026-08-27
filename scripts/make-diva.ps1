# Launch the Agent Diva GUI dev process with its embedded Gateway.
#
# Usage (from repo root):
#   just make-diva
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/make-diva.ps1

param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "[make-diva] $Message" -ForegroundColor Cyan
}

function Assert-Command {
    param(
        [string]$Name,
        [string]$Hint
    )

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Missing command '$Name'. $Hint"
    }
}

function Start-DevWindow {
    param(
        [string]$Title,
        [string]$Workdir,
        [string]$Command
    )

    if (-not (Test-Path -LiteralPath $Workdir)) {
        throw "Missing workdir for [$Title]: $Workdir"
    }

    $escapedWorkdir = $Workdir.Replace("'", "''")
    $escapedTitle = $Title.Replace("'", "''")
    $body = @(
        "Set-Location -LiteralPath '$escapedWorkdir'",
        "Write-Host '[$escapedTitle] starting...' -ForegroundColor Green",
        "Write-Host '$($Command.Replace("'", "''"))' -ForegroundColor DarkGray",
        $Command
    ) -join "; "

    Start-Process -FilePath "powershell.exe" -WorkingDirectory $Workdir -ArgumentList @(
        "-NoExit",
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-Command", $body
    ) | Out-Null

    Write-Step "Opened window [$Title] -> $Command"
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$guiDir = Join-Path $repoRoot "agent-diva-gui"

Assert-Command -Name "pnpm" -Hint "Install pnpm and ensure it is on PATH."
Assert-Command -Name "cargo" -Hint "Install Rust/cargo and ensure it is on PATH."

if (-not (Test-Path -LiteralPath $guiDir)) {
    throw "Missing GUI directory: $guiDir"
}

Write-Step "Repo: $repoRoot"
Start-DevWindow -Title "tauri-dev" -Workdir $guiDir -Command "pnpm tauri dev"
Write-Step "Launched pnpm tauri dev with the embedded Gateway."
Write-Host "Close the window to stop both the GUI and its embedded Gateway." -ForegroundColor DarkGray
