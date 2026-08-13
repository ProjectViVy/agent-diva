# Verification

Passed on 2026-07-29:

- Sandbox suggestion, persistence, corruption, revision conflict, delete, global approval, unsafe approval, and cross-session reuse tests.
- Manager command approval/rule contract tests, including global approval and stale revision conflict.
- Real subprocess smoke `global_rule_reuses_a_real_command_across_sessions`: globally approved `git --version`, executed it, and reused it from another session without a pending request.
- GUI focused approval/rule tests: six passed.
- GUI full Vitest: 53 files and 426 tests passed.
- GUI TypeScript check and production Vite build passed.
- Tauri command tests: 20 passed; focused GUI Clippy passed.
- `just fmt-check`, `just check`, and `just test` all passed.

The desktop interaction walkthrough was not automated because this environment does not provide a safe provider-backed GUI automation fixture. The repeatable real subprocess and contract coverage proves the execution path; manual presentation checks are listed in `acceptance.md`.
