# agent-diva-files

`agent-diva-files` provides the file storage layer used by Agent Diva crates.
It offers content-addressed storage, deduplication, reference counting, and
channel-aware file bookkeeping for local runtimes.

## Scope

- Stores file payloads by SHA-256 hash.
- Tracks references so files are only deleted when no handles remain.
- Supports hooks around store/read/cleanup flows.
- Exposes channel-level file associations on top of the shared storage/index.

This crate is publishable on its own, but it is primarily maintained as part of
the `agent-diva-*` stack. Expect APIs to remain practical for integration, not
as a general-purpose blob store abstraction.

## Minimal usage

```rust,no_run
use agent_diva_files::{FileConfig, FileManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = FileConfig::with_path(PathBuf::from("./data"));
    let _manager = FileManager::new(config).await?;
    Ok(())
}
```

## Relationship to other crates

- `agent-diva-core` depends on this crate for attachment and storage primitives.
- `agent-diva-tools` uses it to expose file-oriented built-in tools.
- `agent-diva-agent` and `agent-diva-nano` consume it indirectly for runtime flows.

## Development notes

- Rust `1.80+`
- Validate with `cargo test -p agent-diva-files`
- Additional docs live in `docs/`
