# Verification

## Automated Tests

- `npm run test -- VrmAppearancePanel.test.ts vrm-animation-scanner.test.ts appearance-config.test.ts pet-config.test.ts DivaVrmAvatar.test.ts DesktopPetOverlay.test.ts DivaPetModelManager.test.ts`
  - Result: passed, 7 test files, 99 tests.

## Build

- `npm run build`
  - Result: passed.
  - Note: Vite reported existing large chunk size warnings after successful build.

## GUI Smoke

- Started Vite dev server with `npm.cmd run dev -- --host 127.0.0.1 --port 4174`.
- Probed `http://127.0.0.1:4174/` while the dev server was running.
  - Result: HTTP 200.

## Notes

- Background dev-server launch did not stay reachable in the sandboxed command runner, so the smoke check used a foreground Vite process in parallel with the HTTP probe.
