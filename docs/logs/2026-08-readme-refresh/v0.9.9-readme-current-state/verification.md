# Verification: README Refresh (EN + ZH)

## Fact-checking performed

1. **Crate list**: cross-checked against root `Cargo.toml`
   `[workspace.members]` (17 members).
2. **CLI commands**: verified against the built binary
   (`agent-diva.exe --help`, `provider --help`, `mask --help`,
   `workspace --help`) — all subcommands referenced in the READMEs exist.
3. **Channels**: `agent-diva-channels/src/lib.rs` default exports vs
   feature-gated retired adapters (2026-08-18 retirement story).
4. **Providers**: counted builtin presets in
   `agent-diva-providers/src/providers.yaml` (45 entries).
5. **Skills paths**: `agent-diva-agent/src/skills.rs` — discovery binds to
   `{config_dir}/skills`; builtin dir defaults to repo `skills/`
   (compile-time fallback; the directory currently does not exist in the
   repo); workspace never scanned.
6. **Doc links**: every path referenced in the new READMEs was checked to
   exist (`docs/README.md`, `docs/architecture/`, `docs/architecture/laputa/`,
   `docs/engineering/CONTRIBUTING.md`, `AGENTS.md`, `AGENTS-ARCH.MD`,
   `TODOLIST.md`, `docs/logs/`, `scripts/package-windows-gui.ps1`,
   `docs/resources/diva.png`, `LICENSE`).
7. **just recipes**: `justfile` recipe list checked (`ci`, `test`,
   `memory-provider-check`, `laputa-clean-break-check`, `bml-boundary-check`,
   `gui-automated-check`, `package-linux`, `build-deb`).

## Not applicable

- `just fmt-check` / `just check` / `just test`: documentation-only change,
  no Rust source touched.
