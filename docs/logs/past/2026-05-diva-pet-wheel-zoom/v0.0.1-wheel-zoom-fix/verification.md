# Verification

1. `pnpm exec vue-tsc --noEmit`
   - Result: passed

2. `pnpm exec vite build`
   - Result: passed

3. `just check`
   - Result: passed

4. `just fmt-check`
   - Result: failed due to pre-existing formatting diffs in `agent-diva-gui/src-tauri/src/commands.rs`
   - Notes: unrelated to this wheel-zoom change

5. `just test`
   - Result: failed due to pre-existing workspace test errors
   - `agent-diva-providers/tests/ollama_streaming.rs`: unresolved import `agent_diva_providers::ollama`
   - `agent-diva-providers/tests/ollama_tools.rs`: unresolved import `agent_diva_providers::ollama`
   - `agent-diva-tools/src/attachment.rs`: unresolved import `agent_diva_files::FileMetadata`

## Smoke expectation

- Open the floating desktop pet window.
- Move the pointer over the pet area and scroll the mouse wheel.
- Confirm the avatar scale changes in 5% steps and remains within the existing 75%-160% bounds.
- Confirm wheel zoom does not trigger when click-through mode is enabled.
- Confirm drag-handle movement still works and is not interrupted by wheel input.
