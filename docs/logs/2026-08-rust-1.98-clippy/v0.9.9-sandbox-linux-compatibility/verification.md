# v0.9.9 Sandbox Linux Compatibility Verification

## Local checks

| Check | Result |
| --- | --- |
| `just fmt-check` | Passed |
| `just check` | Passed with `-D warnings` |
| `just test` | Passed; all workspace tests and doctests passed |
| `cargo test -p agent-diva-sandbox --lib` | Passed; 127 tests passed |
| `git diff --check` | Passed |

The local Windows environment cannot perform a native Linux build: the installed cross-target toolchain has no Linux sysroot/linker for the workspace's `ring` dependency. The Linux-only API migration therefore requires the Ubuntu CI leg for native compilation; the retagged Action is the release gate.

## Failure reproduced and addressed

The prior release run was `32504328675`. Its Ubuntu/macOS failure was caused by stale Landlock and seccompiler API usage, not by the earlier Rust 1.98 Clippy fixes. The source and dependency changes in commit `164fdb4e` address those compiler errors.

## Remaining gate

After the tag is force-updated, inspect the new `v0.9.9` Action and require the Ubuntu, macOS, and Windows jobs to finish successfully before acceptance.
