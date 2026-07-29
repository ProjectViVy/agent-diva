# Verification

Passed on 2026-07-29:

- `pnpm test`: 52 files, 423 tests passed.
- `pnpm exec vitest run src/components/ApprovalBanner.test.ts` twice: all three tests passed on both runs.
- `pnpm run build`: Vue type-check and Vite production build passed.
- `cargo check -p agent-diva-gui`: passed.
- `cargo test -p agent-diva-gui --lib command --no-fail-fast`: 20 tests passed.
- `cargo test -p agent-diva-manager command_approval -- --nocapture`: three HTTP contract tests passed.
- `just fmt-check`, `just check`, `just test`: all workspace gates passed.

The existing real subprocess smoke `shell::tests::approval_resumes_the_same_exec_call_once` also passed through the full workspace gate and proves that approval resumes the original harmless command once.

An interactive provider-backed desktop walkthrough was not automated because the local acceptance path requires configured provider credentials. The repeatable component, Tauri compile, Manager HTTP, and real subprocess evidence above covers the shipped contract; the manual steps remain in `acceptance.md`.
