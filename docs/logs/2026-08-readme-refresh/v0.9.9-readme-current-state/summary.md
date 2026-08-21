# Summary: README Refresh (EN + ZH) to Current Project State

## What changed

Rewrote both root READMEs (`README.md`, `README.zh-CN.md`) to reflect the
current (2026-08) state of the project:

- **Workspace layout**: 9 -> 17 crates; added autodream, laputa, sandbox,
  files, tooling, neuron, service, e2e with accurate one-line roles.
- **Memory/persona section (new)**: BML (profile-local SQLite + FTS5
  authority), Laputa governance (proposals / governed apply / rollback /
  audit / Frozen Core), AutoDream as proposal generator, GUI Persona /
  Evolution surfaces.
- **Feature highlights (new)**: multi-channel gateway, 45+ provider presets,
  skills + skills.sh marketplace, sandbox + HITL approval center, cron,
  external HTTP hook, CLI/TUI/GUI trio.
- **Channels**: defaults now listed as Telegram/Discord/QQ/DingTalk/Feishu/
  Email; retired adapters documented as opt-in Cargo features.
- **CLI**: verified against the real binary (`--help`); added chat,
  approvals, provider, workspace, todo, mask, service; dropped stale claims.
- **Skills**: corrected to `{config_dir}/skills` discovery with the repo
  `skills/` dir as compile-time fallback home (workspace is never scanned).
- **GUI**: features updated (approval center, marketplace, evolution,
  masks/notebook/cron/gateway panel, clean mode); Windows NSIS/MSI packaging
  via `scripts/package-windows-gui.ps1` documented.
- **Requirements**: Rust MSRV corrected 1.70 -> 1.80.
- **Documentation links**: dead links removed (`docs/userguide.md`,
  `docs/dev/*`, `.workspace/agent-diva-docs`, root `CONTRIBUTING.md`);
  replaced with verified paths (`docs/README.md`, `docs/architecture/`,
  `docs/engineering/CONTRIBUTING.md`, `AGENTS.md`, `AGENTS-ARCH.MD`,
  `TODOLIST.md`, `docs/logs/`).

## Impact

- Documentation only; no code, config, or build changes.
