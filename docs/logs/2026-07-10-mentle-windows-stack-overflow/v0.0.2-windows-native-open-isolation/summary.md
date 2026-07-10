# Summary

Windows gateway startup can keep the native Mentle path enabled without `STATUS_STACK_OVERFLOW`.

## Changes

- Call `memtle::init_process_defaults()` before Tokio (CLI) and before palace open (agent runtime).
- Assemble `MemtleToolkit` + `HybridMemoryProvider` + tool definitions on a dedicated 16 MiB stack thread.
- Raise CLI PE main-thread stack and Tokio worker stack to 16 MiB on Windows.
- Remove the previous “disable Mentle on Windows” workaround as the default delivery path.

## Impact

- Mentle Full mode stays available on Windows default binaries.
- Laputa read-only memory injection remains compatible; palace tools still register.
- Known residual: one prompt-rebuild test still lacks the `L2 Palace Memory` block (tracked in `TODOLIST.md`).
