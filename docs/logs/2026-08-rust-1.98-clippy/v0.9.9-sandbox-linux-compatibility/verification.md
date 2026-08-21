# v0.9.9 Sandbox Linux Compatibility Verification

## Local checks

| Check | Result |
| --- | --- |
| `just fmt-check` | Passed |
| `just check` | Passed with `-D warnings` |
| `cargo +1.98.0 clippy --all -- -D warnings` | Passed on Windows with the same Clippy version used by CI |
| `just test` | Passed; all workspace tests and doctests passed |
| `cargo test -p agent-diva-sandbox --lib` | Passed; 127 tests passed |
| `git diff --check` | Passed |

The local Windows environment cannot perform a native Linux build: the installed cross-target toolchain has no Linux sysroot/linker for the workspace's `ring` dependency. The Linux-only API migration therefore requires the Ubuntu CI leg for native compilation; the retagged Action is the release gate.

## Failure reproduced and addressed

The prior release run was `32504328675`. Its Ubuntu/macOS failure was caused by stale Landlock and seccompiler API usage, not by the earlier Rust 1.98 Clippy fixes. Commit `164fdb4e` addressed that API mismatch. The follow-up run `32509518384` then exposed remaining Rust 1.98 sorting lints and Linux-only type/result/unused-parameter errors; commit `6dcca721` addressed those findings. Run `32511048554` exposed six final Linux-only Clippy findings; commit `76716b3a` addressed them. The macOS failure in that run was a crates.io DNS/download outage and is not a source failure.

## Remaining gate

After the tag is force-updated, inspect the new `v0.9.9` Action and require the Ubuntu, macOS, and Windows jobs to finish successfully before acceptance.
