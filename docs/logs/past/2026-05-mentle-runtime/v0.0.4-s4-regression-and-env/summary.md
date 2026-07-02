# Summary

## Changes

- Added S4-A9 regression coverage for `with_toolset()` external-registry
  isolation and subagent prompt Mentle isolation.
- Recorded S4-A10 Mentle feature build prerequisites for Windows, including the
  local LLVM `clang-cl.exe` PATH requirement.
- Added Sprint 4 A9-A11 documentation records for regression, environment, and
  iteration-log evidence.
- Prepared Sprint 4 closure evidence for the A12 architecture review package.

## Impact

- `with_toolset()` now proves that external registries do not become
  runtime-owned custom tools or subagent Mentle capability.
- Windows developers can reproduce Mentle feature verification by prefixing the
  current shell PATH with `C:\Program Files\LLVM\bin` when LLVM is installed but
  not globally discoverable.
- Sprint 4 has a consolidated evidence chain for assembly, cron,
  `with_toolset()`, subagent, prompt, environment, and release-readiness review.
