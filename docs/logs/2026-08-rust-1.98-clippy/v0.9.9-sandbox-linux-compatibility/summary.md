# v0.9.9 Sandbox Linux Compatibility Summary

## Problem

The `v0.9.9` GitHub Action still failed on the Ubuntu and macOS matrix while compiling `agent-diva-sandbox`. The Linux sandbox module used APIs removed or changed by the locked `landlock 0.4.7` and `seccompiler 0.4.0` versions. Windows validation did not compile that Linux-only module.

## Change

- Migrated Landlock ruleset construction to the current `RulesetCreated` and `PathFd` APIs.
- Replaced removed Landlock access constants with the current `AccessFs` bitflags helpers.
- Reworked seccomp filter construction around syscall numbers, `TargetArch`, and `TryInto<BpfProgram>`.
- Added the Linux-only `libc` dependency used for syscall numbers.
- Removed the unused `/proc/version` parser now that ABI detection uses Landlock capability probing directly.

The existing network-filter modes and filesystem policy intent remain unchanged; port-specific filtering continues to be delegated to the existing bubblewrap layer.

## Follow-up CI findings

The first retagged run (`32509518384`) reached the platform builds and exposed two additional Rust 1.98 gates that were invisible to the Windows-only local build: remaining `unnecessary_sort_by` instances across the workspace, and Linux-only Landlock type/result/unused-parameter errors. Those issues were fixed together with the platform corrections.

## Commits

- `164fdb4e` (`fix: restore Linux sandbox compatibility`) updates Landlock/seccompiler compatibility.
- `6dcca721` (`fix: satisfy Rust 1.98 workspace checks`) fixes the remaining workspace sorting lints and the Linux-only compile errors.
