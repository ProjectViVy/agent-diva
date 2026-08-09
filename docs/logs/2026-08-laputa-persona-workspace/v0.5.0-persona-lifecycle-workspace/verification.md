# Verification

Completed:

- `cargo fmt --all -- --check`
- `cargo check -p agent-diva-laputa -p agent-diva-agent -p agent-diva-manager -p agent-diva-gui`
- `cargo test -p agent-diva-laputa frozen_core`
- `cargo test -p agent-diva-agent frozen_core_is_captured_once_and_frozen_for_the_session --lib`
- `cargo test -p agent-diva-manager laputa --lib`
- `pnpm build` in `agent-diva-gui`
- `pnpm test` in `agent-diva-gui`: 60 files, 451 tests passed

GUI smoke coverage is provided by the production Vite build plus mounted component flows for aggregate loading, session-version visualization, JSON validation, proposal creation, cognitive read-only display, and lifecycle proposal rendering.
