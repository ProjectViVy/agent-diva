# Ask Mode Read-only Boundary Verification

Focused coverage verifies:

- Plan, Ask, Agent, and unknown mode admission;
- denial of `execution_start` outside Agent mode;
- the exact five-tool Ask allowlist;
- executor-seam denial for command, file mutation, deletion, planning, TODO,
  cron, and spawn tools;
- Manager normalization of unknown explicit modes to Ask;
- the stable Ask prompt contract.

Validation results:

- focused admission, Ask allowlist, executor-denial, prompt, and Manager mode
  normalization tests passed;
- `just fmt-check`: passed;
- `just check`: passed;
- GUI Vitest: all 432 tests passed;
- GUI production build: passed;
- `cargo test -p agent-diva-agent --lib`: all 377 tests passed;
- `cargo test -p agent-diva-manager --lib`: all 70 tests passed;
- `just test`: blocked during compilation because the currently running
  `target/debug/agent-diva.exe` process holds the output binary and Windows
  denied Cargo's replacement attempt. No test assertion failed before that
  operational block.

The running desktop process is left untouched. Real desktop acceptance requires
restarting it onto the newly built code.
