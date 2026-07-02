# Verification

- Run `npm run build` in `agent-diva-gui`.
- Smoke-check `设置 -> 频道`:
  - Clicking a row selects the channel.
  - Clicking the switch only toggles the enabled draft state.
  - Saving persists the enabled state.
- Smoke-check `设置 -> 供应商`:
  - Existing API keys are loaded from config and shown as masked values.
  - The eye button toggles masked/plain-text display without changing the saved value.
  - Saving updates both runtime config and the local config file.
