# v0.4.5 verification

| Gate | Command | Result |
|------|---------|--------|
| Unit tests | `cargo test -p diva-speech` | 7/7 green |
| Plan-named cases | same run | `credential_unavailable_no_fallback`, `config_revision_conflict`, `asset_traversal_rejected`, `asset_bounds`, `leased_delete_pending` all pass |
| Clippy | `cargo clippy -p diva-speech --all-targets -- -D warnings` | clean |
| Bridge regressions | `just shell-bridge-test` | 7/7 green |
| Boundary gate | `python3 scripts/ci/check_vivy_backend_boundary.py` | clean — allowlist (9 handlers), graph 451 pkgs no legacy crates |
| Format | `cargo fmt -p diva-speech` + `rustfmt` on glue | applied, clean |

## Not runnable on this VM (DN-8C obligations)
- `just shell-clippy` / `cargo check` on `src-tauri`: requires pkg-config
  + glib/webkit dev libs (DN-0S recorded). Shell glue is therefore
  uncompiled here — must compile on Windows acceptance host.
- Real OS keyring roundtrip: dev VM exposes no default store
  (`credential_unavailable` path proven; roundtrip covered by injected
  `SecretStore`).
