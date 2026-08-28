# agent-diva-files

## OVERVIEW

Content-addressed file storage with SHA256 deduplication, reference counting, SQLite indexing, and a hook system for compression/encryption/permission checks.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| Main API | `src/manager.rs` (`FileManager`) |
| Storage backend trait / local impl | `src/backend.rs` (`StorageBackend`, `LocalStorageBackend`) |
| File handle + metadata | `src/handle.rs` (`FileHandle`) |
| SQLite index | `src/index.rs` (`FileIndex`, `SqliteIndex`) |
| Raw storage + SHA256 | `src/storage.rs` (`FileStorage`) |
| Channel file bookkeeping | `src/channel.rs` (`ChannelManager`, `ChannelFileInfo`) |
| Lifecycle hooks | `src/hooks.rs` (`HookRegistry`, `MetadataHook`, `ReadHook`, `StorageHook`, `CleanupHook`) |
| Configuration | `src/config.rs` (`FileConfig`, `CleanupConfig`, `CleanupStrategy`) |
| Error type | `src/lib.rs` (`FileError`, `Result`) |

## CONVENTIONS

- Files are stored by SHA256 hash; `FileHandle` is the public reference.
- Reference counting is managed inside `FileManager`; callers drop handles to release refs.
- New storage backends implement `StorageBackend` and are wired in `FileConfig`.
- Hooks are registered via `HookRegistry` and run on store/read/cleanup.

## ANTI-PATTERNS

- Do not bypass `FileManager` to write or delete payloads directly.
- Do not store non-file attachment bytes in `agent-diva-core` memory types.
- Do not leak absolute storage paths through public APIs.

## NOTES

- `agent-diva-core` depends on this crate for attachment primitives.
- `agent-diva-tools` consumes it for file-oriented built-in tools.
