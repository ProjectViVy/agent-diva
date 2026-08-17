# Verification

## Automated checks

- `npm test -- --run src/components/ChatView.test.ts`: passed, 11/11 tests.
  Covers preserving an upward reading position during streaming and
  `ask_user` updates, resuming near the bottom, and forcing the latest position
  after send/session switch.
- `npm test`: passed, 59 files and 430 tests.
- `npm run build`: passed (`vue-tsc --noEmit` and Vite production build).
  Vite retained its existing large-chunk advisory.
- `just gui-automated-check`: passed, including the 430 GUI tests, production
  frontend build, and Tauri `cargo check`.
- `just fmt-check`: passed.
- `just check`: passed. The existing `imap-proto` future-incompatibility notice
  remains informational.
- `just test`: passed using
  `CARGO_TARGET_DIR=C:\tmp\agent-diva-chat-scroll-test` and
  `CARGO_BUILD_JOBS=1`.

## Environment notes

- The first default-target `just test` attempt could not replace the running
  `target\debug\agent-diva.exe`; the existing application process was not
  stopped.
- The first isolated cold build exhausted available memory. Reusing the same
  target directory with one build job completed the full workspace suite.

## GUI smoke

On 2026-08-17 the user manually exercised the long-message chat case and
reported that the result looked correct: scrolling upward to read older content
was no longer visibly forced back to the bottom.
