# Acceptance

## Criteria

- S4-A9 records a regression set for assembly, cron/default rebuild,
  `with_toolset()`, subagent, and prompt routing.
- `with_toolset()` proves registry-only activation and does not convert supplied
  external tools into runtime-owned custom tools.
- Subagent config, registry assembly, and prompt template remain isolated by
  default; registry assembly does not inherit parent custom tools.
- S4-A10 records Windows Mentle feature prerequisites and distinguishes PATH
  setup from code failure.
- Mentle feature checks pass when `C:\Program Files\LLVM\bin` is prefixed to the
  current shell PATH.
- S4-A11 consolidates summary, verification, acceptance, and release notes for
  Sprint 4 closure.

## Result

Accepted for Sprint 4 closure evidence.

The remaining Sprint 4 decision is A12 architecture sign-off against the
complete evidence package.
