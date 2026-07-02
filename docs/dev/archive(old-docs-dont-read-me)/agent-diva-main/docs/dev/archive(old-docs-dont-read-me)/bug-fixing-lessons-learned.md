# Bug Fixing Experience Summary

**Date**: 2026-03-31
**Context**: Fixed two critical bugs in Agent Diva that affected core functionality

---

## Overview

This document summarizes the lessons learned from debugging and fixing two interconnected bugs:

1. **GUI Connection Issue**: GUI showing "Offline" status and "Bad Gateway" errors
2. **File Upload System**: AI unable to read uploaded files despite successful upload

The debugging process revealed subtle issues in Windows networking behavior and path consistency across different components.

---

## Bug 1: GUI Connection Issue

### Symptoms
- GUI displayed "Offline" status persistently
- Sending messages resulted in "Bad Gateway" errors
- Issue was NOT related to API keys or LLM provider configuration

### Root Cause

**Windows System HTTP Proxy Interception**

On Windows systems with HTTP proxy configured (common in corporate environments or with tools like Clash/V2Ray), the system proxy was intercepting localhost requests:

```
Frontend (GUI) → System Proxy → localhost:3000
                     ↓
              502 Bad Gateway (proxy rejects localhost)
```

The `reqwest` HTTP client by default respects system proxy settings, causing all GUI-to-Manager API calls to route through the system proxy, which then failed to handle localhost addresses properly.

### The Fix

**File**: `agent-diva-gui/src-tauri/src/app_state.rs`

```rust
// Before: Default client that respects system proxy
let client = reqwest::Client::new();

// After: Client that bypasses proxy for localhost
let client = reqwest::Client::builder()
    .no_proxy()  // Critical: bypass system proxy for localhost APIs
    .build()
    .expect("reqwest client for local Manager API");
```

Additionally, changed the server binding to explicit IPv4:

**File**: `agent-diva-manager/src/server.rs`

```rust
// Before: Dual-stack IPv6/IPv4 binding
let addr = SocketAddr::from(([127, 0, 0, 1], port)); // IPv4 only

// Actually changed FROM:
// let addr = SocketAddr::from(([::1], port)); // IPv6
// TO:
let addr = SocketAddr::from(([127, 0, 0, 1], port)); // IPv4
```

### Key Insight

> **Local services should always use `.no_proxy()`** when using reqwest or similar HTTP clients. This prevents system proxy interference and ensures reliable localhost communication.

---

## Bug 2: File Upload System - AI Cannot Read Files

### Symptoms
- File upload appeared successful (GUI showed progress, no errors)
- AI responses indicated no file content was accessible
- No error messages in logs about file not found

### Root Cause

**Path Inconsistency Between Upload and Read Operations**

The file storage system had a critical path mismatch:

| Component | Storage Path |
|-----------|-------------|
| Upload (`file_service.rs`) | `%LOCALAPPDATA%/agent-diva/files/` |
| Read (`agent_loop/loop_turn.rs`) | `~/.agent-diva/files/` (or current dir) |

The upload service correctly used `dirs::data_local_dir()` to determine the storage location:

```rust
// file_service.rs - Upload path (CORRECT)
fn data_dir() -> anyhow::Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| anyhow!("failed to find local data directory"))?;
    Ok(base.join("agent-diva").join("files"))
}
```

But the read operation in the agent loop used `FileConfig::default()` which calculated a different path:

```rust
// loop_turn.rs - Read path (WRONG - before fix)
let file_config = FileConfig::default(); // Returns ~/.agent-diva/files on Unix
```

### The Fix

**File**: `agent-diva-agent/src/agent_loop/loop_turn.rs`

```rust
async fn load_attachment_contents(&self, file_ids: &[String]) -> Result<String, Box<dyn std::error::Error>> {
    // Use the SAME path calculation as file_service.rs
    let storage_path = dirs::data_local_dir()
        .map(|p| p.join("agent-diva").join("files"))
        .unwrap_or_else(|| PathBuf::from(".agent-diva/files"));

    let config = FileConfig::with_path(storage_path);
    let file_manager = FileManager::new(config).await?;
    // ... rest of function
}
```

**File**: `agent-diva-agent/Cargo.toml`

```toml
[dependencies]
# Added to match file_service.rs dependency
dirs = { workspace = true }
```

### Key Insight

> **Path calculations MUST be centralized or use the same logic.** Having different components compute the same path differently leads to silent failures where files are written to one location but read from another.

---

## File Attachment Data Flow

Understanding the complete flow helped identify where the disconnect occurred:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         FILE UPLOAD FLOW                                     │
└─────────────────────────────────────────────────────────────────────────────┘

1. FRONTEND (GUI/Tauri)
   └─► User selects file → UploadRequest { message_id, file_data }
           │
           ▼ HTTP POST /api/upload

2. MANAGER (handlers.rs:674)
   └─► upload_file_handler()
       ├─► Validates request
       ├─► Calls file_service.save_file()
       └─► Returns { status, attachment }
           │
           ▼

3. FILE SERVICE (file_service.rs)
   └─► save_file()
       ├─► Calculates SHA256 hash of content
       ├─► Stores at: %LOCALAPPDATA%/agent-diva/files/<hash>
       ├─► Creates FileAttachment with hash as file_id
       └─► Returns FileAttachment { file_id, name, mime_type, size }
           │
           ▼ HTTP POST /api/chat (with attachments)

4. CHAT HANDLER (handlers.rs)
   └─► chat_handler()
       ├─► Extracts attachments from request
       ├─► Stores message with attachment metadata
       └─► Triggers agent processing
           │
           ▼ Message Bus

5. AGENT LOOP (loop_turn.rs)
   └─► process_turn()
       ├─► load_attachment_contents(file_ids)
       │   ├─► MUST use SAME path as file_service.rs
       │   └─► Reads file content by hash
       ├─► Includes content in LLM prompt
       └─► Sends to LLM provider
```

### Critical Observation

The file system uses **content-addressed storage** (SHA256 hash as filename), which provides:
- Automatic deduplication
- Content integrity verification
- Simple cache invalidation

However, this design requires all components to agree on the storage root directory.

---

## Lessons Learned

### 1. Windows-Specific Networking Behavior

- Windows HTTP proxies can intercept localhost traffic
- Always use `.no_proxy()` for local service communication
- IPv4 vs IPv6 binding can matter on some systems

### 2. Path Consistency

- Never compute the same path differently in different modules
- Use a shared configuration function or constant
- The `dirs` crate is essential for cross-platform path handling

### 3. Silent Failures Are Worst

- The file read failure was silent - no error was logged
- The file existed, just in a different location
- Consider adding validation: "File written to X but attempted read from Y"

### 4. Debugging Strategy That Worked

1. **Confirm the problem**: Verify upload actually creates a file
2. **Trace the data flow**: Follow file from upload to AI consumption
3. **Compare implementations**: Check how different components calculate paths
4. **Add logging**: Instrument both sides of the operation
5. **Test the fix**: Generate test file and verify end-to-end

---

## Testing Verification

After fixes, verified with:

```bash
# 1. Create test file
echo "这是一个测试文件内容，用于验证文件上传和读取功能是否正常工作" > test_upload.txt

# 2. Upload via API
curl -X POST http://localhost:3000/api/upload \
  -F "message_id=test-123" \
  -F "file=@test_upload.txt"

# 3. Verify AI can read and summarize content
# Result: AI correctly summarized the Chinese test content
```

---

## Related Files

| File | Purpose | Key Fix |
|------|---------|---------|
| `agent-diva-gui/src-tauri/src/app_state.rs` | GUI HTTP client | Added `.no_proxy()` |
| `agent-diva-manager/src/server.rs` | Manager server binding | Changed to IPv4 only |
| `agent-diva-agent/src/agent_loop/loop_turn.rs` | File reading | Fixed path calculation |
| `agent-diva-agent/Cargo.toml` | Dependencies | Added `dirs` crate |
| `agent-diva-manager/src/file_service.rs` | File storage | Reference implementation |

---

## Prevention Recommendations

1. **Centralize path configuration**: Create a shared `paths.rs` module that all components import
2. **Add integration tests**: Test file upload → read round-trip
3. **Document proxy requirements**: Add note about `.no_proxy()` for local development
4. **Validate file existence**: Add explicit checks with informative error messages
5. **Log path decisions**: Log the resolved paths at runtime for debugging
