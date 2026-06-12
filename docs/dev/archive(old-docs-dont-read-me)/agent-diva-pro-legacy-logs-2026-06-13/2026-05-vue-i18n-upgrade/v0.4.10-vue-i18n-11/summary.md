# Summary

## Changes

- Upgraded `agent-diva-gui` from `vue-i18n` 9.x to 11.4.4 to remove the deprecated package warning during `pnpm install`.
- Updated `pnpm-lock.yaml` so the deprecated `vue-i18n@9.14.5` entry is no longer resolved.
- Aligned GUI `ToolsConfigShape` declarations around the shared `MentleToolConfigShape` type to keep `vue-tsc` passing after validation exposed existing type drift.

## Impact

- Scope is limited to the GUI package dependency graph and TypeScript-only GUI type compatibility.
- Runtime i18n usage remains on the existing composition API path: `createI18n({ legacy: false })` and `useI18n()`.
- `vue-i18n` 11.x requires Node >= 22 according to package metadata; current validation used Node v25.9.0.
