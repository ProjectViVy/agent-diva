# Verification

Validated the Laputa service and manager integration with focused Rust checks.

- `cargo test -p agent-diva-laputa`
- `cargo test -p agent-diva-manager build_router_exposes_laputa_routes`
- `cargo check -p agent-diva-manager`

GUI compile checks were attempted but remained blocked by pre-existing `agent-diva-sandbox` errors unrelated to this story:

- `agent-diva-sandbox/src/exec_policy.rs`: missing `File::lock_exclusive`
- `agent-diva-sandbox/src/platform/macos.rs`: `bool` / `&bool` match arm mismatch
