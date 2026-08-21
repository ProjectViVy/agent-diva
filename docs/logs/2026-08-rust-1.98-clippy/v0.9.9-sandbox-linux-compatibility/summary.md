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

## Commit

The implementation is recorded in commit `164fdb4e` (`fix: restore Linux sandbox compatibility`).
