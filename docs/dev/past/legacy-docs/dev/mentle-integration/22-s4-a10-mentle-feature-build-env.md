# Sprint 4 A10: Mentle Feature Build Environment

## Purpose

S4-A10 records the reproducible environment requirement for the Mentle feature
lane. The goal is to distinguish code regressions from host setup gaps,
especially on Windows where `memtle` depends on a native C/C++ toolchain.

## Environment Contract

| Requirement | Status |
|---|---|
| Rust toolchain for default lane | Rust `1.80+` |
| Rust toolchain for Mentle lane | Rust `1.88+` |
| Current local Rust | `rustc 1.93.0 (254b59607 2026-01-19)` |
| Windows native compiler | `clang-cl.exe` |
| Current local compiler path | `C:\Program Files\LLVM\bin\clang-cl.exe` |
| Current shell PATH before fix | `where.exe clang-cl` did not find the tool |
| Reproducible local fix | Prefix current shell PATH with `C:\Program Files\LLVM\bin` |
| Frozen Mentle dependency | `memtle 0.1.2`, `default-features = false` |

## Windows PATH Setup

For a PowerShell session that has LLVM installed but not on PATH:

```powershell
$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH
where.exe clang-cl
```

Expected result:

```text
C:\Program Files\LLVM\bin\clang-cl.exe
```

If `clang-cl.exe` is absent, record the Mentle feature lane as `blocked` with
the canonical Windows signature from the Sprint 3 baseline:

```text
error occurred in cc-rs: failed to find tool "clang-cl.exe": program not found
```

## Verification Commands

After adding LLVM to PATH in the current shell:

```powershell
cargo check -p agent-diva-agent --features mentle
cargo test -p agent-diva-agent --features mentle mentle
cargo test -p agent-diva-core --features mentle memory
```

Static dependency policy was checked with repository search:

```text
memtle = { version = "0.1.2", default-features = false }
```

No `memtle` `path`, `git`, or `[patch.crates-io]` override was found.

## Result

S4-A10 is passed for this host after prefixing the current shell PATH with
`C:\Program Files\LLVM\bin`.

The earlier missing-tool state was an environment PATH issue, not a repository
regression.

## Acceptance

S4-A10 is accepted when Mentle feature verification is recorded as one of:

- `passed`, with Rust version, compiler path, and commands
- `blocked`, with the missing prerequisite and canonical error signature
- `failed`, only when the toolchain is present and the command reports a code or
  test regression
