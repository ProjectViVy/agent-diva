# Acceptance

## User Acceptance Steps

1. Open `agent-diva-gui`.
2. Run `pnpm install`.
3. Confirm there is no `vue-i18n@9.14.5` deprecation warning.
4. Run `pnpm test` and confirm the GUI test suite passes.
5. Run `pnpm run build` and confirm the production build completes.

## Expected Result

- `vue-i18n` resolves to 11.4.4.
- The GUI test suite passes.
- The GUI production build passes.
