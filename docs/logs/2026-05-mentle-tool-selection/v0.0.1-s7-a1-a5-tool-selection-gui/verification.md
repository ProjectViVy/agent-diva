# Verification

## Commands

Run from `agent-diva/` workspace root:

```bash
just fmt-check          # pass (after cargo fmt)
just check              # pass
cargo test -p agent-diva-core mentle_config   # 2/2 pass
cargo test -p agent-diva-agent tool_config::mentle   # 3/3 pass
cargo test -p agent-diva-agent set_mentle_prompt_state   # 1/1 pass
cargo test -p agent-diva-agent --features mentle test_mentle_tool_filter   # 2/2 pass
```

Note: `cargo test -p agent-diva-manager` currently fails in an unrelated
`file_service.rs` assertion (`file_name` vs `filename`); manager library
build and clippy pass via `just check`.

## GUI smoke (manual)

1. Start GUI gateway runtime.
2. Open Settings → General → Mentle Memory Tools.
3. Confirm current config loads (default off).
4. Enable Mentle, choose `read_only`, save, reload settings page, confirm values persist.
5. Switch to `custom`, select tools from checklist, save, reload, confirm checklist state.
6. Confirm note about runtime application is visible.

## Expected behavior

- `off` / disabled: no `memtle_*` tools and no Mentle prompt text.
- `read_only`: only read/status subset when toolkit provides them.
- `full`: all valid dynamic tools from toolkit definitions.
- `custom`: intersection of `allowed_tools` and discovered toolkit tools.
- Missing `memtle_status` after filtering keeps Mentle prompt routing inactive.
