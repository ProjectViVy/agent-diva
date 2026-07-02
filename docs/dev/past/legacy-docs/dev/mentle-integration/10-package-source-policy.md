# Mentle Package Source and Version Policy

## 1. Decision

Agent-Diva's Mentle integration must use the published Cargo package `memtle` as the dependency source. The integration must not depend on a local sibling repository such as `../mentle`, any other local path-based checkout, or a git override.

The current frozen implementation target is:

- crate: `memtle`
- source: `crates.io`
- version: exact `0.1.2`
- features: `default-features = false`

This is a hard constraint for both implementation and verification.

## 2. Allowed and Forbidden Forms

Allowed:

- Cargo dependency resolved by crate version from `crates.io`
- exact version pin `0.1.2` in the workspace dependency declaration
- optional feature-gated dependency on the published package
- CI and local verification against the same published package source

Forbidden:

- `path = "../mentle"`
- any other `path = "...memtle..."`
- `git = "..."` for `memtle`
- `[patch.crates-io]` override for `memtle`
- workspace/path dependency coupling to a local `mentle/` directory
- build scripts or ad hoc local override steps that make production behavior depend on the developer machine layout
- automatic drift to `0.1.x` or newer versions without a dedicated upgrade review

## 3. Why This Is Required

### 3.1 Release Discipline

The integration needs a stable version boundary. A published Cargo package gives Agent-Diva:

- explicit version pinning
- reproducible CI behavior
- release notes that map to a real upstream package version
- a rollback target when integration regressions appear

### 3.2 Environment Consistency

If the implementation consumes a local `mentle/` checkout, behavior can silently vary by workstation state. That breaks:

- reviewability
- CI parity
- release reproducibility
- defect triage

### 3.3 Architectural Boundary

Mentle is an upstream dependency, not part of the Agent-Diva monorepo. The integration plan therefore needs a package boundary rather than a local source-code boundary.

### 3.4 Build Surface Control

`memtle 0.1.2` is authored with `edition = "2024"` and `rust-version = "1.88"`. Its default feature set includes CLI-facing behavior that Agent-Diva does not need for in-process embedding.

Freezing the integration to `default-features = false` keeps the default Agent-Diva build isolated from unnecessary compile surface, and keeps the Mentle build path explicit.

## 4. Impact on the Delivery Plan

This decision changes the planning baseline in these ways:

- Sprint 1 dependency foundation must explicitly freeze the Cargo package source policy.
- Sprint 2 must freeze and record the exact crate version, feature surface, and upgrade policy.
- Sprint 2 and Sprint 3 technical design must assume a published package surface, not unpublished local APIs.
- Acceptance checks must verify that no local path dependency was introduced during implementation.

## 5. Required Technical Follow-Ups

Before implementation is considered production-ready, the team must document:

1. the exact crate name and frozen version
2. the Rust toolchain requirement imposed by that published package
3. the feature flags used on the package
4. the upgrade and rollback policy for future Mentle releases
5. the CI checks that prove no local path dependency was used

Sprint 2 records this downstream constraint set in [11-s2-a3-published-crate-constraints.md](./11-s2-a3-published-crate-constraints.md).

## 6. Frozen Upgrade and Rollback Policy

### 6.1 Upgrade Policy

- `memtle` upgrades must be submitted as a dedicated change, not piggybacked onto runtime or memory behavior work.
- The default branch must stay on exact `0.1.2` until a separate review approves a new version.
- A Mentle upgrade review must re-run:
  - `cargo check -p agent-diva-agent --features mentle`
  - `cargo test -p agent-diva-core --features mentle memory`
  - `cargo test -p agent-diva-agent --features mentle mentle`
- The review must reconfirm that `default-features = false` still exposes the toolkit and tool schema surfaces required by Agent-Diva.
- The review must reconfirm that the upstream package still resolves from `crates.io` and still matches the documented Rust/toolchain floor.

### 6.2 Rollback Policy

- If a future `memtle` upgrade regresses build stability or toolkit compatibility, Agent-Diva must roll back to the last known-good published version.
- The current rollback target is `memtle 0.1.2`.
- Rollback must not rely on swapping to a local sibling checkout.

## 7. Build Preconditions

The Mentle feature path has stricter environment requirements than the default Agent-Diva workspace build:

- Rust compiler: `1.88.0` or newer
- Windows native toolchain: a working C/C++ compiler toolchain must be present
- CI baseline for Windows: the repository currently installs LLVM so that `clang-cl.exe` is available for native dependencies
- Upstream package shape: `edition = "2024"`, `default = ["cli", "mcp"]`, while Agent-Diva embeds it with `default-features = false`

This requirement is not theoretical. Local verification on `2026-05-24` failed before Mentle-specific code checks completed because `clang-cl.exe` was missing from the Windows environment.

## 8. Acceptance Criteria

The package source policy is satisfied only when all of the following are true:

- Cargo manifests reference the published Mentle package by versioned dependency
- the workspace dependency declaration remains `memtle = { version = "0.1.2", default-features = false }`
- no manifest under Agent-Diva references local `mentle/` via `path =`
- no manifest or workspace override rewires `memtle` through `git` or `[patch.crates-io]`
- CI validates the intended feature path against the published package source
- implementation docs and project planning docs both reflect this rule

## 9. Review Note

This policy is a prerequisite, not an optional optimization. Any implementation branch that consumes a local `mentle/` checkout should be treated as non-production and blocked from merge until the dependency source is corrected.
