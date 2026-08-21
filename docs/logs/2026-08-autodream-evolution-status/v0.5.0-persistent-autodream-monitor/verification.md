# Verification

- `npx vue-tsc --noEmit` — passed.
- `npm test -- --run src/components/EvolutionView.test.ts` — 9 passed.
- `npm test -- --run src/components/ChatView.test.ts src/components/chat/ChatGovernanceCard.test.ts` — 21 passed.
- `npm test -- --run src/components/EvolutionView.test.ts src/components/ChatView.test.ts src/components/chat/ChatGovernanceCard.test.ts` — 30 passed.
- `npm test -- --run` — 472 passed; 2 pre-existing suites (`NormalMode.test.ts`, `memory/MemoryView.test.ts`) could not load because a running process held `node_modules/@marijn/find-cluster-break/src/index.js` (`EPERM`).
- `npm run build` — type check passed; Vite bundling was blocked by the same Windows `node_modules` file-lock family (`@codemirror/language/dist/index.js`, `EPERM`).
- `cargo check --manifest-path agent-diva-gui/src-tauri/Cargo.toml` — passed.
- `just fmt-check` — passed.
- `just check` — passed before the final frontend-only adjustments.
- `just test` — blocked while Cargo attempted to replace a locked `target/debug/agent-diva.exe` (`拒绝访问`, Windows error 5); no test assertion failure was reported.
- GUI smoke: `npm run dev -- --host 127.0.0.1` served `http://127.0.0.1:1420/` with HTTP 200.
- `git diff --check` — passed.

The remaining release follow-up is to rerun the production build and full Rust test gate after the process holding the Windows binaries/dependencies is closed.
