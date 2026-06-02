# Provider Selection Follow-ups

This document tracks the unfinished follow-up items after the `2026-03 provider-selection-fix` iteration.

## Pending items

- Add a manual model entry control in the GUI provider settings page.
  - Current state: CLI `onboard` already supports falling back to manual model input.
  - Gap: GUI can preserve and display an unknown current model, but it still lacks a dedicated input for adding a new provider-owned model directly from the UI.

- Centralize provider resolution logic into a shared provider service.
  - Current state: this iteration added explicit `agents.defaults.provider` and reduced model-name guessing.
  - Gap: CLI, Manager, Tauri, and GUI still each own part of the provider/config mapping logic.
  - Goal: converge on one provider resolution/access/catalog service so future provider/model changes do not require parallel edits.

- Reduce hardcoded provider-slot dispatch.
  - Current state: explicit provider selection is now persisted and used in more places.
  - Gap: `provider_config_by_name`-style `match` dispatch still exists across crates.
  - Goal: remove duplicated slot mapping and make provider access less brittle for future provider expansion.

- Add richer GUI regression coverage for provider/model selection.
  - Current state: Rust-side config tests and Vue type-check passed for this iteration.
  - Gap: there is no automated GUI regression covering:
    - default DeepSeek startup display,
    - provider switch without accidental invalid save,
    - unknown model display/reselection.

- Re-run full GUI build/smoke validation in an environment that permits Vite/esbuild child process spawning.
  - Current state: `npx.cmd vue-tsc --noEmit` passed.
  - Gap: `npm.cmd run build` was blocked by `spawn EPERM` in the current environment, so full GUI bundle validation is still pending.

- Re-run full workspace validation in an environment with sufficient Windows pagefile/resources.
  - Current state: targeted CLI tests passed and `cargo check` passed.
  - Gap: `cargo test --all` was not usable as a signal in this environment because of `os error 1455` and unrelated existing failures outside this iteration's scope.
