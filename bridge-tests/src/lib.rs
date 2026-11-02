// Compile the production storage/process code without Tauri or a sidecar artifact.
#[path = "../../src-tauri/src/storage.rs"]
pub mod storage;
#[path = "../../src-tauri/src/process.rs"]
pub mod process;
#[path = "../../src-tauri/src/diagnostics.rs"]
pub mod diagnostics;
#[path = "../../src-tauri/src/bridge.rs"]
pub mod bridge;
