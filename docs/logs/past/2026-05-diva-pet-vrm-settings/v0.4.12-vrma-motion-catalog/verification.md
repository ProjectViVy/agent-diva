# Verification

## Commands

- `pnpm -C agent-diva-gui test`: passed, 22 files / 273 tests.
- `pnpm -C agent-diva-gui build`: passed.
- `just fmt-check`: passed.
- `just check`: passed. Rust reported a dependency future-incompat warning for `imap-proto v0.10.2`.
- `just test`: failed in pre-existing Rust test compile errors unrelated to this GUI animation catalog change:
  - `agent-diva-providers/tests/ollama_streaming.rs` and `ollama_tools.rs` import `agent_diva_providers::ollama`, which is not exported.
  - `agent-diva-tools/src/attachment.rs` test imports `agent_diva_files::FileMetadata`, but the available path is `agent_diva_files::handle::FileMetadata`.

## Notes

- Vite serves files from `agent-diva-gui/public`, so animations added only under `avatar-runtime-vrm/public` are not visible to the main GUI until synced or copied.
