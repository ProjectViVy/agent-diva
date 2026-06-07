# Sprint 6 A5: CI, Build, and Dependency Source Validation

## Purpose

This record closes the Sprint 6 RC build, CI, and dependency-source validation
lane for Mentle x Agent-Diva.

Sprint 6 remains RC and handoff only. This report does not add Mentle features,
change dependency policy, alter CI behavior, or reopen Sprint 7 GUI/tool
selection work.

## Scope and References

Validation followed:

- workspace instructions in `AGENTS.md`
- Agent-Diva instructions in `agent-diva/AGENTS.md`
- Memtle instructions in `memtle/AGENTS.md` and `memtle/STYLEGUIDE.md`
- S6-A1 frozen RC baseline in
  `docs/dev/mentle-integration/29-s6-a1-rc-scope-baseline.md`
- Sprint 5 failure and CI baseline in
  `docs/dev/mentle-integration/26-s5-a1-failure-validation-matrix.md`,
  `27-s5-a2-a6-failure-and-ci-hardening.md`, and
  `28-s5-a7-sprint5-review-package.md`

## CI and Build Matrix Review

Current CI coverage matches the RC baseline:

| Lane | CI job or command | Coverage result |
|---|---|---|
| Default Rust lane | `.github/workflows/ci.yml` `rust-check` on `ubuntu-latest`, `windows-latest`, `macos-latest` | Runs `just fmt-check`, `just check`, `just build`, then `just sprint5-default-check` |
| Mentle feature lane | `.github/workflows/ci.yml` `mentle-check` on `ubuntu-latest`, `windows-latest`, `macos-latest` | Uses Rust `1.88.0`, verifies package policy, runs Mentle check and targeted Mentle tests |
| Windows native prerequisite | CI Windows `mentle-check` | Installs LLVM with Chocolatey and appends `C:\Program Files\LLVM\bin` to `GITHUB_PATH` |
| GUI build lane | `.github/workflows/ci.yml` `gui-build` on Linux, Windows, macOS | Remains separate from Mentle feature validation and depends on `rust-check` |
| Full tests and coverage | `workflow_dispatch` inputs | Remain manual gates, consistent with the known full-workspace test exception from Sprint 5/S6-A1 |

Local verification commands remain aligned with CI:

- `just sprint5-default-check` mirrors the default RC regression lane.
- `just mentle-check` runs package policy, Mentle feature check, and targeted
  Mentle memory/agent tests.
- `scripts/ci/verify_mentle_package_policy.py` is shared by local and CI checks.

## Dependency Source Compliance

The published Cargo package policy continues to hold:

| Check | Result |
|---|---|
| Workspace dependency declaration | `memtle = { version = "0.1.2", default-features = false }` |
| Lockfile source | `memtle 0.1.2` resolves from `registry+https://github.com/rust-lang/crates.io-index` |
| Local path dependency search | No `memtle` path dependency found |
| Git dependency search | No `memtle` git dependency found |
| `[patch.crates-io]` override search | No `memtle` crates.io patch override found |
| Policy script | Passed: `Mentle package policy verified: crates.io memtle 0.1.2` |

Feature gating also remains intact:

| Dependency tree check | Result |
|---|---|
| `cargo tree -p agent-diva-agent --no-default-features` | No `memtle` in the default dependency tree |
| `cargo tree -p agent-diva-agent --features mentle` | Contains `memtle v0.1.2` |

## Local Validation Results

Host/toolchain observations:

- OS: Windows workspace shell.
- `rustc 1.93.0`.
- `cargo 1.93.0`.
- `just 1.46.0`.
- `Python 3.13.2`.
- `clang-cl.exe` was available at `C:\Program Files\LLVM\bin` but not initially
  on `PATH`.

Commands executed:

| Command | Result |
|---|---|
| `python scripts/ci/verify_mentle_package_policy.py` | Passed |
| `cargo check -p agent-diva-agent --no-default-features` | Passed |
| `just sprint5-default-check` | Passed |
| `$env:PATH = "C:\Program Files\LLVM\bin;" + $env:PATH; just mentle-check` | Passed |
| `cargo tree -p agent-diva-agent --no-default-features` with `memtle` search | Passed: default tree has no `memtle` |
| `cargo tree -p agent-diva-agent --features mentle` with `memtle v0.1.2` search | Passed |

Observed targeted test counts:

- `just sprint5-default-check` passed all listed default-lane regressions,
  including `with_toolset`, subagent isolation, cron/default rebuild, prefetch
  failure, and consolidation sync-failure cases.
- `cargo test -p agent-diva-core --features mentle memory` passed `39` tests.
- `cargo test -p agent-diva-agent --features mentle mentle` passed `20` tests.

## Blocker and Non-Blocker Assessment

Blockers:

- None found in this validation pass.

Non-blockers:

- Local Windows shells still need `clang-cl.exe` on `PATH` for the Mentle feature
  lane. This host had LLVM installed at `C:\Program Files\LLVM\bin`; adding that
  directory to `PATH` allowed `just mentle-check` to pass.
- Full workspace `just test` was not rerun for S6-A5. S6-A1 and Sprint 5 already
  classify the known provider export failure around `agent_diva_providers::ollama`
  as outside the Mentle RC gate unless Sprint 6 explicitly promotes full
  workspace tests to blocker status.

## Conclusion

S6-A5 accepts the current RC build and dependency-source posture:

- Default build/check does not pull `memtle`.
- Mentle remains behind the explicit `mentle` Cargo feature.
- Mentle feature-lane native prerequisite is documented and works locally when
  `clang-cl.exe` is on `PATH`.
- Agent-Diva continues to consume published crates.io `memtle 0.1.2` with
  `default-features = false`.
- No local `memtle/` path dependency, git dependency, or crates.io patch override
  was found.
- CI covers the RC-required default, Mentle feature, package-policy, and platform
  build lanes.
- Local validation commands and CI entry points remain aligned for the RC scope.
