# Verification

## Commands

- `npm run test -- NormalMode.test.ts`
  - Result: Passed. 4 tests passed.
  - Coverage: normal page topbar visibility, pet page focus class/sidebar collapse, pet view sidebar toggle, topbar remains hidden in pet page, topbar restores after leaving pet page.

- `npm run build`
  - Result: Failed due existing TypeScript errors outside this focused change path.
  - Latest observed remaining errors include: `ChatView.vue` argument mismatch, unused declarations in `DecisionCard.vue` and several settings components, `NotebookView.vue` unused/timer typing issues, `SettingsView.vue`/settings tool config type mismatch, `TodoCard.vue` import path, and `DivaPetView.vue` timer typing.
  - The earlier `NormalMode.vue`/`App.vue` `mentle` config compatibility errors were cleared by normalizing `toolsConfig` before passing it to `SettingsView`.

## GUI Smoke

- Started local Vite server directly with `node .\node_modules\vite\bin\vite.js --host 127.0.0.1 --port 4176`.
- Confirmed `http://127.0.0.1:4176` returned HTTP 200.
- In-app Browser smoke was blocked because the Browser backend returned `Browser is not available: iab`.
- Manual visual browser verification was therefore not completed in this session.
- Stopped the local Vite process after verification.

## Notes

Focus Mode is now limited to the pet page. The pet page never renders `topbar`; expanding the sidebar from the pet menu button leaves the topbar absent.
