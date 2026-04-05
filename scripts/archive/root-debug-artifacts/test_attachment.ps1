# Test attachment flow end-to-end

Write-Host "=== Testing Agent Diva Attachment Flow ===" -ForegroundColor Cyan

# 1. Create a test file
$testContent = "This is a test file for attachment debugging."
$testFile = "$env:TEMP\attachment_test.txt"
$testContent | Out-File -FilePath $testFile -Encoding UTF8
Write-Host "Created test file: $testFile" -ForegroundColor Green

# 2. Test upload
Write-Host "`n1. Testing file upload..." -ForegroundColor Yellow
$uploadUrl = "http://127.0.0.1:3000/api/files/upload"

$boundary = [System.Guid]::NewGuid().ToString()
$headers = @{
    "Content-Type" = "multipart/form-data; boundary=$boundary"
}

$body = @"
--$boundary
Content-Disposition: form-data; name="channel"

gui
--$boundary
Content-Disposition: form-data; name="message_id"

test_msg_123
--$boundary
Content-Disposition: form-data; name="file"; filename="attachment_test.txt"
Content-Type: text/plain

$testContent
--$boundary--
"@

try {
    $response = Invoke-RestMethod -Uri $uploadUrl -Method POST -Headers $headers -Body $body
    Write-Host "Upload response:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3

    if ($response.status -eq "ok" -and $response.attachment) {
        $fileId = $response.attachment.file_id
        Write-Host "`nFile uploaded successfully!" -ForegroundColor Green
        Write-Host "File ID: $fileId" -ForegroundColor Yellow
        Write-Host "Filename: $($response.attachment.filename)" -ForegroundColor White
        Write-Host "Size: $($response.attachment.size)" -ForegroundColor White

        # 3. Check if file exists in storage
        $storagePath = "$env:LOCALAPPDATA\agent-diva\files"
        Write-Host "`n2. Checking storage location..." -ForegroundColor Yellow
        Write-Host "Expected storage path: $storagePath" -ForegroundColor White

        if (Test-Path $storagePath) {
            Write-Host "Storage directory exists" -ForegroundColor Green
            $dataDir = Join-Path $storagePath "data"
            if (Test-Path $dataDir) {
                $files = Get-ChildItem $dataDir -Recurse -File | Select-Object -First 5
                Write-Host "Files in data directory:" -ForegroundColor Green
                $files | ForEach-Object { Write-Host "  $($_.FullName)" -ForegroundColor Gray }
            }
        } else {
            Write-Host "Storage directory NOT found!" -ForegroundColor Red
        }

        # 4. Test chat with attachment
        Write-Host "`n3. Testing chat with attachment..." -ForegroundColor Yellow
        $chatUrl = "http://127.0.0.1:3000/api/chat"
        $chatBody = @{
            message = "Please read this file and tell me its content"
            channel = "gui"
            chat_id = "test_chat"
            attachments = @($fileId)
        } | ConvertTo-Json

        Write-Host "Chat request body:" -ForegroundColor Gray
        $chatBody

        Write-Host "`nSending chat request (check gateway logs for agent response)..." -ForegroundColor Cyan
        try {
            # This will stream SSE events, we just want to see if it starts
            $response = Invoke-WebRequest -Uri $chatUrl -Method POST -ContentType "application/json" -Body $chatBody -TimeoutSec 5
            Write-Host "Chat request sent successfully" -ForegroundColor Green
        } catch {
            Write-Host "Chat request result: $($_.Exception.Message)" -ForegroundColor Yellow
        }

        Write-Host "`n=== Check gateway logs for: ===" -ForegroundColor Cyan
        Write-Host "1. 'Loading attachments from:' path" -ForegroundColor White
        Write-Host "2. 'File IDs to load:' should contain: $fileId" -ForegroundColor White
        Write-Host "3. Any errors about file not found" -ForegroundColor White

    } else {
        Write-Host "Upload failed or no attachment in response" -ForegroundColor Red
        $response | ConvertTo-Json -Depth 3
    }
} catch {
    Write-Host "Upload request failed: $_" -ForegroundColor Red
    Write-Host "Make sure gateway is running on http://127.0.0.1:3000" -ForegroundColor Red
}

# Cleanup
Remove-Item $testFile -ErrorAction SilentlyContinue

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
Write-Host "Check gateway logs at: $env:USERPROFILE\.agent-diva\logs\gateway.log" -ForegroundColor Yellow
