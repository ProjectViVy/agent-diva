# Release

## Method

- No separate deployment was performed in this iteration.
- The change is delivered through repository files: `agent-diva-gui/package.json`, `agent-diva-gui/pnpm-lock.yaml`, and small GUI TypeScript type fixes.

## Notes

- Consumers should run `pnpm install` in `agent-diva-gui` with Node >= 22 because `vue-i18n` 11.4.4 declares that engine requirement.
