# Test script to verify file upload and read flow

Write-Host "=== Testing Agent Diva File Upload ===" -ForegroundColor Cyan

# 1. Create a test file
$testContent = "这是一个测试文件，用于验证Agent Diva文件上传系统。If you can read this, the file system is working correctly!"
$testFile = "$env:TEMP\test_upload.txt"
$testContent | Out-File -FilePath $testFile -Encoding UTF8
Write-Host "Created test file: $testFile" -ForegroundColor Green

# 2. Test upload via API
Write-Host "`n1. Testing API upload..." -ForegroundColor Yellow
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

test_message_123
--$boundary
Content-Disposition: form-data; name="file"; filename="test_upload.txt"
Content-Type: text/plain

$testContent
--$boundary--
"@

try {
    $response = Invoke-RestMethod -Uri $uploadUrl -Method POST -Headers $headers -Body $body
    Write-Host "Upload response:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3

    if ($response.status -eq "ok") {
        $fileId = $response.attachment.file_id
        Write-Host "`nFile uploaded successfully! File ID: $fileId" -ForegroundColor Green

        # 3. Test chat with attachment
        Write-Host "`n2. Testing chat with attachment..." -ForegroundColor Yellow
        $chatUrl = "http://127.0.0.1:3000/api/chat"
        $chatBody = @{
            message = "请读取并总结这个文件的内容"
            channel = "gui"
            chat_id = "test_chat"
            attachments = @($fileId)
        } | ConvertTo-Json

        $chatHeaders = @{
            "Content-Type" = "application/json"
        }

        Write-Host "Sending chat request with attachment..." -ForegroundColor Cyan
        try {
            $chatResponse = Invoke-RestMethod -Uri $chatUrl -Method POST -Headers $chatHeaders -Body $chatBody
            Write-Host "Chat request sent. Check gateway logs for AI response." -ForegroundColor Green
        } catch {
            Write-Host "Chat request failed: $_" -ForegroundColor Red
        }
    } else {
        Write-Host "Upload failed: $($response.message)" -ForegroundColor Red
    }
} catch {
    Write-Host "Upload request failed: $_" -ForegroundColor Red
    Write-Host "Make sure gateway is running on http://127.0.0.1:3000" -ForegroundColor Red
}

# Cleanup
Remove-Item $testFile -ErrorAction SilentlyContinue
Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
Write-Host "`nTo see detailed logs, check:" -ForegroundColor Yellow
Write-Host "$env:USERPROFILE\.agent-diva\logs\gateway.log" -ForegroundColor White
