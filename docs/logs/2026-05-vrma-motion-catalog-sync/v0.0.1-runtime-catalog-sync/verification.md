# Verification

## Completed

- `pnpm -C agent-diva-gui test` passed: 23 test files, 275 tests.
- `pnpm -C agent-diva-gui build` passed. Vite reported the existing large chunk warning.
- Runtime demo animation directory coverage check passed: no differences between GUI `.vrma` files and `avatar-runtime-vrm/public/vrm/animations`.
- `just fmt-check` passed.
- `just check` passed. Rust emitted an existing future-incompatibility warning for `imap-proto v0.10.2`.
- `just test` failed in existing Rust test compilation paths unrelated to this GUI/runtime catalog change:
  - `agent-diva-providers/tests/ollama_streaming.rs` and `ollama_tools.rs`: unresolved `agent_diva_providers::ollama`.
  - `agent-diva-tools/src/attachment.rs`: test import should use `agent_diva_files::handle::FileMetadata`.
  - `agent-diva-manager/src/file_service.rs`: test still references `attachment.file_name` instead of `attachment.filename`.

## Notes

- Manual smoke should preview `Clapping`, `Goodbye`, and `Thinking`, then select `LookAround`, `Relax`, and `Sleepy` as idle motions.
