# M6 Verification

## Commands

- `npm run build` in `agent-diva-gui`: passed.
  - Initial run failed because `vue-tsc` was missing from local dependencies.
  - Ran `npm install` to restore `node_modules`; lockfile had no retained diff after refresh.
  - Final build passed with the existing Vite large chunk warning.
- `cargo test -p agent-diva-core session`: passed, 18 tests.
- `cargo test -p agent-diva-agent vision`: passed, 6 tests.
- `just fmt-check`: passed.
- `just check`: passed.
- `just test`: failed on existing unrelated test `skills::tests::test_default_builtin_dir_loads_skills`.
  - Observed result: 67 passed, 1 failed in `agent-diva-agent`.
  - Failure assertion: no builtin skill source found.
  - This is outside the M6 GUI attachment surface.

## GUI Smoke

- Started Vite dev server on `http://127.0.0.1:5173/`.
- Opened the GUI in the in-app browser.
- Confirmed the chat shell rendered with title `Agent Diva`, model selector `deepseek-chat`, history button, welcome message, attachment button, textbox, stop button, and disabled send button.
- Closed the temporary Vite process after smoke.

## Manual Acceptance Still Recommended

- In a full Tauri runtime, upload a PNG/JPEG/WebP and confirm image chip rendering in the composer.
- Send a message with an image attachment and confirm the user bubble preserves the attachment chip.
- Reload a session with stored attachment metadata and confirm the history bubble restores the chip.
