# Summary: Global Version Bump 0.5.0 -> 0.9.9

## What changed

- Root `[workspace.package]` version in `Cargo.toml`: `0.5.0` -> `0.9.9`.
- All 17 workspace member crates' `Cargo.toml` `version` fields: `0.5.0` -> `0.9.9`
  (core, agent, providers, channels, tools, files, tooling, neuron, manager,
  autodream, laputa, sandbox, cli, service, gui/src-tauri, migration, e2e).
- All internal path-dependency version pins (`version = "0.5.0"`) across crate
  manifests and the root `[workspace.dependencies]` entries updated to `0.9.9`.
- `Cargo.lock` regenerated via cargo resolution (17 entries now `0.9.9`).
- `AGENTS.md` current-repository-state version line and snapshot date refreshed.

## Impact

- Version metadata only: no source, behavior, config, or API change.
- Windows installer filenames will carry `0.9.9` on the next packaging run
  (tauri.conf.json has no explicit version field; it derives from the
  src-tauri crate version).
- `agent-diva-gui/package.json` keeps its independent npm version (`0.4.10`).
- `.workspace/agent-diva-nano` nested workspace is untouched (separate
  template-line versioning).
