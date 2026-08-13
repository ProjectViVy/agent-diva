# Verification

- Run `npm run build` in `agent-diva-gui` to cover Vue type-checking and production build output.
- Smoke-check the settings `频道` panel:
  - Clicking a channel row only changes selection.
  - Clicking `激活` or `取消激活` only toggles that channel's enabled state.
  - The detail header status pill updates correctly and is no longer clickable.
