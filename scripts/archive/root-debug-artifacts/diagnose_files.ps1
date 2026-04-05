# Agent Diva File System Diagnostic Script
Write-Host "=== Agent Diva File System Diagnostic ===" -ForegroundColor Cyan
Write-Host ""

# 1. Check config directory
$configDir = "$env:USERPROFILE\.agent-diva"
Write-Host "1. Config Directory: $configDir" -ForegroundColor Yellow
if (Test-Path $configDir) {
    Write-Host "   Status: EXISTS" -ForegroundColor Green
    Get-ChildItem $configDir -Directory | ForEach-Object {
        Write-Host "   - $($_.Name)/"
    }
} else {
    Write-Host "   Status: NOT FOUND" -ForegroundColor Red
}
Write-Host ""

# 2. Check log directory
$logDir = "$configDir\logs"
Write-Host "2. Log Directory: $logDir" -ForegroundColor Yellow
if (Test-Path $logDir) {
    Write-Host "   Status: EXISTS" -ForegroundColor Green
    $logFiles = Get-ChildItem $logDir -File | Sort-Object LastWriteTime -Descending | Select-Object -First 5
    foreach ($file in $logFiles) {
        Write-Host "   - $($file.Name) ($([math]::Round($file.Length/1KB, 2)) KB)"
    }
} else {
    Write-Host "   Status: NOT FOUND" -ForegroundColor Red
}
Write-Host ""

# 3. Check file storage directory
$storageDir = "$env:LOCALAPPDATA\agent-diva\files"
Write-Host "3. File Storage Directory: $storageDir" -ForegroundColor Yellow
if (Test-Path $storageDir) {
    Write-Host "   Status: EXISTS" -ForegroundColor Green

    # Check SQLite database
    $dbFile = "$storageDir\index.db"
    if (Test-Path $dbFile) {
        Write-Host "   - index.db: EXISTS ($([math]::Round((Get-Item $dbFile).Length/1KB, 2)) KB)" -ForegroundColor Green

        # Query database
        try {
            $result = sqlite3 $dbFile "SELECT COUNT(*) as count FROM files WHERE deleted_at IS NULL;" 2>$null
            Write-Host "   - Active files in database: $result" -ForegroundColor Green
        } catch {
            Write-Host "   - Could not query database (sqlite3 not installed?)" -ForegroundColor Yellow
        }
    } else {
        Write-Host "   - index.db: NOT FOUND" -ForegroundColor Red
    }

    # Check data directory
    $dataDir = "$storageDir\data"
    if (Test-Path $dataDir) {
        $subDirs = Get-ChildItem $dataDir -Directory | Measure-Object
        Write-Host "   - data/ subdirectories: $($subDirs.Count)" -ForegroundColor Green
    } else {
        Write-Host "   - data/: NOT FOUND" -ForegroundColor Red
    }
} else {
    Write-Host "   Status: NOT FOUND" -ForegroundColor Red
}
Write-Host ""

# 4. Check gateway process
Write-Host "4. Gateway Process" -ForegroundColor Yellow
$gatewayProcess = Get-Process -Name "agent-diva" -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -like "*gateway*" }
if ($gatewayProcess) {
    Write-Host "   Status: RUNNING (PID: $($gatewayProcess.Id))" -ForegroundColor Green
} else {
    Write-Host "   Status: NOT RUNNING" -ForegroundColor Red
}
Write-Host ""

# 5. Check port 3000
Write-Host "5. Port 3000" -ForegroundColor Yellow
try {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 3000)
    $listener.Start()
    $listener.Stop()
    Write-Host "   Status: AVAILABLE (no process listening)" -ForegroundColor Yellow
} catch {
    Write-Host "   Status: IN USE (process is listening)" -ForegroundColor Green
}
Write-Host ""

# 6. Test API connectivity
Write-Host "6. API Connectivity Test" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/health" -Method GET -TimeoutSec 5
    Write-Host "   Status: CONNECTED" -ForegroundColor Green
    Write-Host "   Response: $response" -ForegroundColor Gray
} catch {
    Write-Host "   Status: FAILED - $_" -ForegroundColor Red
}
Write-Host ""

# 7. Sample file content test
Write-Host "7. Sample File Test" -ForegroundColor Yellow
$testContent = "Test content for debugging"
$testFile = "$env:TEMP\agent_diva_test.txt"
$testContent | Out-File -FilePath $testFile -Encoding UTF8 -Force

# Calculate SHA256
$hash = (Get-FileHash $testFile -Algorithm SHA256).Hash.ToLower()
$expectedFileName = $hash.Substring(2)  # Remove first 2 chars for subdir
$expectedSubDir = $hash.Substring(0, 2)
$expectedPath = "$storageDir\data\$expectedSubDir\$expectedFileName"

Write-Host "   Test file hash: sha256:$hash"
Write-Host "   Expected storage location: $expectedPath"

if (Test-Path $expectedPath) {
    Write-Host "   Status: File exists at expected location" -ForegroundColor Green
} else {
    Write-Host "   Status: File NOT at expected location (will be created on upload)" -ForegroundColor Yellow
}

Remove-Item $testFile -Force
Write-Host ""

Write-Host "=== Diagnostic Complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "To view logs in real-time, run:" -ForegroundColor Yellow
Write-Host "Get-Content '$logDir\gateway.log' -Wait -Tail 10" -ForegroundColor White
