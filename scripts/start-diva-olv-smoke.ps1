param(
    [switch]$SkipGui,
    [switch]$SkipBrowser,
    [switch]$VerboseOlv
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "[diva-olv-smoke] $Message" -ForegroundColor Cyan
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

function Assert-Path {
    param([string]$Path)

    if (-not (Test-Path $Path)) {
        throw "Missing file or directory: $Path"
    }
}

function Start-Window {
    param(
        [string]$Title,
        [string]$Workdir,
        [string]$Command
    )

    $escapedWorkdir = $Workdir.Replace("'", "''")
    $banner = "Write-Host '[$Title] starting...' -ForegroundColor Green"
    $body = @(
        "Set-Location '$escapedWorkdir'",
        $banner,
        $Command
    ) -join "; "

    Start-Process powershell -ArgumentList @(
        "-NoExit",
        "-ExecutionPolicy", "Bypass",
        "-Command", $body
    ) | Out-Null
}

function Wait-HttpReady {
    param(
        [string]$Url,
        [int]$TimeoutSeconds = 60
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        try {
            Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 3 | Out-Null
            return $true
        } catch {
            Start-Sleep -Seconds 2
        }
    }

    return $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$guiDir = Join-Path $repoRoot "agent-diva-gui"
$olvDir = Join-Path $repoRoot ".workspace\olv-diva\Open-LLM-VTuber"
$divaConfigPath = Join-Path $env:USERPROFILE ".agent-diva\config.json"
$olvConfigPath = Join-Path $olvDir "conf.yaml"

Write-Step "Checking environment and config"
Assert-Command -Name "cargo" -Hint "Install Rust stable first."
Assert-Command -Name "npm" -Hint "Install Node.js and GUI dependencies first."
Assert-Command -Name "uv" -Hint "Install uv and OLV Python dependencies first."
Assert-Path -Path $repoRoot
Assert-Path -Path $guiDir
Assert-Path -Path $olvDir
Assert-Path -Path $divaConfigPath
Assert-Path -Path $olvConfigPath

$divaConfig = Get-Content $divaConfigPath -Raw | ConvertFrom-Json
$neuroLink = $divaConfig.channels.'neuro-link'
if (-not $neuroLink.enabled) {
    throw "Diva neuro-link is disabled: $divaConfigPath"
}

if ($neuroLink.allow_from -notcontains "olv-avatar") {
    Write-Warning "Diva allow_from does not include 'olv-avatar'. OLV registration may fail."
}

Write-Step "Starting Diva gateway"
Start-Window -Title "diva-gateway" -Workdir $repoRoot -Command "cargo run --bin agent-diva -- gateway"

Write-Step "Starting Open-LLM-VTuber server"
$olvCommand = if ($VerboseOlv) { "uv run run_server.py --verbose" } else { "uv run run_server.py" }
Start-Window -Title "olv-server" -Workdir $olvDir -Command $olvCommand

if (-not $SkipGui) {
    Write-Step "Starting Diva GUI"
    Start-Window -Title "diva-gui" -Workdir $guiDir -Command "npm run tauri dev"
}

if (-not $SkipBrowser) {
    Write-Step "Waiting for OLV frontend"
    $olvUrl = "http://127.0.0.1:12393"
    if (Wait-HttpReady -Url $olvUrl -TimeoutSeconds 90) {
        Write-Step "Opening OLV page: $olvUrl"
        Start-Process $olvUrl | Out-Null
    } else {
        Write-Warning "OLV HTTP service did not become ready in time. Check the olv-server window."
    }
}

Write-Host ""
Write-Host "Launch sequence has been triggered." -ForegroundColor Green
Write-Host "Suggested verification steps:" -ForegroundColor Yellow
Write-Host "1. Confirm the OLV page is open and connected."
Write-Host "2. Send one message from Diva GUI."
Write-Host "3. Check whether OLV receives speak events and drives Live2D/TTS."
Write-Host ""
Write-Host "Optional switches:" -ForegroundColor Yellow
Write-Host "  -SkipGui      Start gateway + OLV only"
Write-Host "  -SkipBrowser  Do not open the OLV page automatically"
Write-Host "  -VerboseOlv   Start OLV with verbose logging"
