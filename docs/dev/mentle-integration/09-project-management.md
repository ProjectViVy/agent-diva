# Mentle x Agent-Diva Project Management

## 1. Objective

This document tracks the agile delivery plan for production-grade Mentle integration into Agent-Diva. The target outcome is:

- Keep `MEMORY.md` as L0/L1 Compass memory.
- Add Mentle/Memtle as L2 Palace memory.
- Inject `memtle_*` tools dynamically.
- Route memory lifecycle through `HybridMemoryProvider`.
- Keep startup, cron rebuild, `with_toolset()`, subagent spawning, feature gates, and CI in a production-safe state.
- Consume Mentle from a published Cargo package, not from a local sibling `mentle/` path dependency.

Current delivery state:

- Sprint 1 is complete and has passed review.
- Sprint 2 is complete through the S2-A8 review package and Sprint 3 interface baseline.
- Sprint 2 entry constraints now include the published `memtle 0.1.2` feature and toolchain record.
- Cargo feature gates, CI package policy checks, and provisional Mentle runtime/provider code already exist in the repository and must be treated as alignment targets for Sprint 3 work.
- Sprint 3 must consume the interface baseline in [12-s2-a8-sprint2-review-and-s3-interface-baseline.md](./12-s2-a8-sprint2-review-and-s3-interface-baseline.md), the adapter freeze in [13-s3-a1-memtle-toolkit-tool-interface.md](./13-s3-a1-memtle-toolkit-tool-interface.md), the dynamic registration model in [14-s3-a2-dynamic-tool-registration-model.md](./14-s3-a2-dynamic-tool-registration-model.md), the toolkit error mapping in [15-s3-a3-toolkit-error-mapping.md](./15-s3-a3-toolkit-error-mapping.md), the runtime assembly boundary in [16-s3-a4-a6-mentle-runtime-assembly.md](./16-s3-a4-a6-mentle-runtime-assembly.md), the test baseline in [17-s3-a7-test-and-verification-baseline.md](./17-s3-a7-test-and-verification-baseline.md), and the review package in [18-s3-a8-sprint3-review-package.md](./18-s3-a8-sprint3-review-package.md).
- Sprint 4 is complete for AgentLoop hardening through [19-s4-a1-sprint4-entry-audit.md](./19-s4-a1-sprint4-entry-audit.md), the S4-A8 adapter/runtime compatibility review in [20-s4-a8-adapter-runtime-compatibility-review.md](./20-s4-a8-adapter-runtime-compatibility-review.md), the S4-A9 through S4-A12 closure records, and the v0.0.2/v0.0.3/v0.0.4 Sprint 4 iteration logs.
- Sprint 5 is complete for failure modes and CI hardening through [26-s5-a1-failure-validation-matrix.md](./26-s5-a1-failure-validation-matrix.md), [27-s5-a2-a6-failure-and-ci-hardening.md](./27-s5-a2-a6-failure-and-ci-hardening.md), and [28-s5-a7-sprint5-review-package.md](./28-s5-a7-sprint5-review-package.md). It consumes the Sprint 4 review package without reopening Sprint 7 tool-selection or GUI scope.
- Sprint 7 is planned as a post-RC enhancement through [25-s7-a1-mentle-tool-selection-and-gui-controls.md](./25-s7-a1-mentle-tool-selection-and-gui-controls.md). It may start only after Sprint 6 RC acceptance and must not block the Sprint 1-6 production integration baseline.

## 1.1 Package Source Policy

This initiative must consume Mentle through Cargo package resolution. The implementation path is explicitly:

- use a published crate dependency from Cargo/crates.io
- do not use a local sibling directory such as `../mentle`
- do not introduce workspace/path dependency coupling to the local `mentle/` checkout

Reason:

- production builds need a reproducible dependency source
- versioning, CI, and release validation need an explicit crate version boundary
- local path coupling would make the integration plan inconsistent with Agent-Diva's package and release discipline

## 2. Team

| Code | Role | Responsibility |
|---|---|---|
| A-ARCH | Architecture Lead | Integration boundaries, design review, technical sign-off |
| A-CORE | Core Memory Engineer | `HybridMemoryProvider`, cached snapshot, prefetch, sync flow |
| A-LOOP | Agent Runtime Engineer | `AgentLoop`, `MentleRuntime`, cron rebuild, `with_toolset()`, subagent policy |
| A-MEM | Mentle Adapter Engineer | `MemtleToolkitTool`, tool definition mapping, toolkit call handling |
| A-QA | Quality Engineer | Unit tests, integration tests, regression, acceptance evidence |
| A-DEVOPS | Build Engineer | Cargo features, toolchain matrix, CI jobs |
| A-GUI | GUI Engineer | `agent-diva-gui` settings UX, persisted controls, GUI smoke testing |
| A-DOC | Documentation Engineer | Planning docs, release notes, handoff package |

## 3. WBS

| WBS | Work Package | Main Deliverables | Owner | Exit Criteria |
|---|---|---|---|---|
| 1.0 | Dependency and feature foundation | Optional `mentle` feature path, crates.io package source policy, default build isolation, Rust 1.88+ Mentle build path | A-DEVOPS | Build strategy frozen |
| 2.0 | Hybrid memory core | `HybridMemoryProvider`, cached L2 snapshot, async lifecycle hooks | A-CORE | No blocking DB call in `system_prompt_block()` |
| 3.0 | Mentle tool adapter | `MemtleToolkitTool`, dynamic tool loading, toolkit call wrapper | A-MEM | Key `memtle_*` tools available dynamically |
| 4.0 | Runtime assembly | `MentleRuntime`, unified `build_agent_tools`, initial registry assembly | A-LOOP | One assembly path for initial and custom tools |
| 5.0 | Context and prompt routing | `with_mentle(active)`, dual-track memory routing, graceful fallback | A-LOOP | Prompt only advertises active capability |
| 6.0 | Background and advanced entrypoints | cron rebuild, `with_toolset()` strategy, subagent isolation | A-LOOP | No prompt/tool split in advanced paths |
| 7.0 | Quality and verification | Unit, integration, failure, and CI coverage | A-QA | Acceptance suite green |
| 8.0 | Release readiness | Risk register, release checklist, handoff docs | A-DOC | RC package ready |
| 9.0 | Mentle tool selection and GUI controls | Tool-selection config, filtered `memtle_*` assembly, GUI General Settings controls | A-GUI/A-LOOP | Post-RC tool subsets persist and match prompt exposure |

## 4. Sprint Roadmap

Assumption: one-week Sprints.

| Sprint | Theme | Status | Main Outcome |
|---|---|---|---|
| Sprint 1 | Build foundation and feature gates | Completed | Default build isolation and Mentle feature boundary defined |
| Sprint 2 | Core hybrid memory provider | Completed | `HybridMemoryProvider` contract, cached snapshot path, and published-package implementation constraints packaged for Sprint 3 |
| Sprint 3 | Tool adapter and runtime helper | Completed | Dynamic `memtle_*` tools and `MentleRuntime` consume the S2-A8 baseline and S3-A1 adapter freeze |
| Sprint 4 | AgentLoop, cron, `with_toolset()`, subagent | Completed | Assembly closure and memory-safety behavior |
| Sprint 5 | Failure modes and CI hardening | Completed | Regression and downgrade confidence |
| Sprint 6 | RC and handoff | Planned | Release candidate package |
| Sprint 7 | Mentle tool selection and GUI controls | Planned | Optional post-RC tool-level activation and GUI control |

### Sprint 1 Review

Completed activities:

- Defined the workspace-level Mentle dependency strategy.
- Defined the package source rule: published Cargo package only, no local `mentle/` path dependency.
- Defined `agent-diva-core/mentle` feature isolation.
- Defined `agent-diva-agent/mentle` feature propagation.
- Defined the CI split between default build and Mentle build.
- Completed Sprint 1 acceptance record and risk refresh.

Review outcome:

- Sprint 1 is accepted.
- Sprint 2 may start.
- Remaining risk from Sprint 1 is the real compile-surface validation for `default-features = false`.

### Sprint 2 Entry Criteria

- Sprint 1 review accepted.
- Provider contract is frozen for `HybridMemoryProvider`.
- `default-features = false` assumptions are recorded as explicit technical checks.
- The Cargo dependency policy is frozen to published `crates.io` `memtle 0.1.2` with no local path/git override.
- The published-crate feature and toolchain contract is recorded in [11-s2-a3-published-crate-constraints.md](./11-s2-a3-published-crate-constraints.md).
- The team agrees that no prompt path may block on async DB calls.

### Sprint 2 Review Package

Sprint 2 closes through [12-s2-a8-sprint2-review-and-s3-interface-baseline.md](./12-s2-a8-sprint2-review-and-s3-interface-baseline.md).

The review package freezes:

- the `MemoryProvider` lifecycle contract consumed by runtime assembly
- the `HybridMemoryProvider` cached snapshot and async lifecycle behavior
- the published `memtle 0.1.2` package/toolkit boundary
- the dynamic `tool_definitions()` and `call_json()` adapter contract for Sprint 3
- the verification commands and policy checks required before Sprint 3 sign-off

Review outcome:

- Sprint 2 is accepted.
- Sprint 3 may proceed using the S2-A8 baseline and the S3-A1 adapter freeze.

### Sprint 3 A1 Interface Freeze

S3-A1 freezes the `MemtleToolkitTool` adapter boundary in [13-s3-a1-memtle-toolkit-tool-interface.md](./13-s3-a1-memtle-toolkit-tool-interface.md).

The freeze records:

- `MemtleToolkit::tool_definitions()` as the only tool metadata source
- `name`, `description`, and `inputSchema` as the only definition fields consumed by the adapter
- `Tool::to_schema()` as the provider-facing schema conversion path
- `MemtleToolkit::call_json(name, args).await` as the only execution path
- `Arc<tokio::sync::Mutex<MemtleToolkit>>` as the shared toolkit handle shape
- `ToolError::ExecutionFailed` as the unified toolkit call error shape
- skipped-and-warned invalid definitions instead of disabling the whole Mentle runtime

### Sprint 3 A2 Dynamic Tool Registration Model

S3-A2 freezes the dynamic `memtle_*` registration model in [14-s3-a2-dynamic-tool-registration-model.md](./14-s3-a2-dynamic-tool-registration-model.md).

The model records:

- `MemtleToolkit::open(path).await` as the runtime activation gate
- `MemtleToolkit::tool_definitions()` as the only dynamic schema source
- per-definition mapping of only `name`, `description`, and `inputSchema`
- `inputSchema` as the Agent-Diva `Tool::parameters()` payload, converted provider-side by existing `Tool::to_schema()`
- `MemtleToolkit::call_json(name, args).await` as the fixed execution path
- invalid definitions skipped individually with warnings, without downgrading an otherwise-open Mentle runtime
- S3-A3's dependency on a reusable `Vec<Arc<dyn Tool>>` custom tool vector for startup, cron rebuild, `with_toolset()`, and active prompt routing work

### Sprint 3 A3 Toolkit Error Mapping

S3-A3 freezes the toolkit call error mapping in [15-s3-a3-toolkit-error-mapping.md](./15-s3-a3-toolkit-error-mapping.md).

The mapping records:

- startup/open failures disable Mentle and keep Markdown memory as fallback
- invalid definitions are skipped individually with structured warnings
- `call_json` transport errors and error payloads become `ToolError::ExecutionFailed`
- internal logs carry phase, category, and fallback action
- startup prompt routing is active only when dynamic registration exposes `memtle_status`

### Sprint 3 A4-A6 Runtime Assembly Boundary

S3-A4 through S3-A6 freeze the runtime assembly boundary in [16-s3-a4-a6-mentle-runtime-assembly.md](./16-s3-a4-a6-mentle-runtime-assembly.md).

The boundary records:

- `MentleRuntime` as the internal owner of toolkit, runtime provider, custom tools, and active flag
- `runtime.active()` as the prompt-routing activation rule
- `build_agent_tools(...)` as the single registry assembly helper
- preserved custom tool reuse across startup and later rebuild paths

### Sprint 3 A7 Test and Verification Baseline

S3-A7 freezes the minimum verification set in [17-s3-a7-test-and-verification-baseline.md](./17-s3-a7-test-and-verification-baseline.md).

The baseline records:

- the minimum default-lane, dynamic-tool, inactive-runtime, and rebuild-path verification set
- the required command set and policy checks
- the interpretation rule for `passed`, `failed`, and environment-`blocked` verification outcomes
- the current Windows Mentle lane block on missing `clang-cl.exe`

### Sprint 3 A8 Review Package

S3-A8 closes Sprint 3 through [18-s3-a8-sprint3-review-package.md](./18-s3-a8-sprint3-review-package.md).

The review package records:

- the full Sprint 3 interface baseline consumed by Sprint 4
- the prompt-routing activation anchor on `memtle_status`
- the verification status summary carried into review
- the unresolved operational risks that remain visible to Sprint 4

### Sprint 4 AgentLoop Hardening

Sprint 4 closes the AgentLoop hardening scope through
[19-s4-a1-sprint4-entry-audit.md](./19-s4-a1-sprint4-entry-audit.md) and the
iteration log at
`docs/logs/2026-05-mentle-runtime/v0.0.2-s4-agentloop-hardening/`.

The hardening work confirms:

- initial AgentLoop assembly consumes active `MentleRuntime` helper state
- `build_agent_tools(...)` remains the single registry assembly helper
- startup cron rebuild and `register_default_tools(...)` preserve Mentle custom
  tools
- `with_toolset()` disables Mentle prompt routing unless the supplied registry
  contains `memtle_status`
- subagent configuration and registry assembly do not inherit Mentle long-term
  memory capability by default

### Sprint 4 A8 Adapter/Runtime Compatibility Review

S4-A8 closes through
[20-s4-a8-adapter-runtime-compatibility-review.md](./20-s4-a8-adapter-runtime-compatibility-review.md)
and the iteration log at
`docs/logs/2026-05-mentle-runtime/v0.0.3-s4-a8-compatibility-review/`.

The review confirms:

- S4-A2 through S4-A6 did not break the S3 dynamic tool definition convention
- `MemtleToolkit::call_json(name, args).await` remains the only generic adapter
  execution path
- toolkit transport and payload failures still map to
  `ToolError::ExecutionFailed`
- invalid definitions remain local skips and do not deactivate an otherwise-open
  runtime
- prompt routing remains anchored on actual `memtle_status` availability

### Sprint 4 A9-A12 Closure

Sprint 4 closes its remaining regression, environment, documentation, and
architecture sign-off work through:

- [21-s4-a9-regression-test-baseline.md](./21-s4-a9-regression-test-baseline.md)
- [22-s4-a10-mentle-feature-build-env.md](./22-s4-a10-mentle-feature-build-env.md)
- [23-s4-a11-sprint4-iteration-log.md](./23-s4-a11-sprint4-iteration-log.md)
- [24-s4-a12-sprint4-review-package.md](./24-s4-a12-sprint4-review-package.md)
- `docs/logs/2026-05-mentle-runtime/v0.0.4-s4-regression-and-env/`

The closure confirms:

- assembly, cron/default rebuild, `with_toolset()`, subagent config, subagent
  registry, and subagent prompt paths have targeted regression evidence
- local Windows Mentle feature verification passes when LLVM's
  `C:\Program Files\LLVM\bin` is added to the current shell PATH
- no Sprint 4 assembly path keeps an undocumented prompt/tool mismatch open

### Sprint 5 Failure Modes and CI Hardening

Sprint 5 closes through the failure validation matrix in
[26-s5-a1-failure-validation-matrix.md](./26-s5-a1-failure-validation-matrix.md),
the hardening record in
[27-s5-a2-a6-failure-and-ci-hardening.md](./27-s5-a2-a6-failure-and-ci-hardening.md),
and the review package in
[28-s5-a7-sprint5-review-package.md](./28-s5-a7-sprint5-review-package.md).

The hardening scope covers:

- Mentle open/startup failure downgrade to Markdown memory
- query/recall failure behavior during prefetch
- `sync_turn()` write failure behavior, including failed `memtle_diary_write`
- dynamic tool definition skip behavior
- runtime active/inactive state consistency with actual registry contents
- cron/default rebuild preservation
- `with_toolset()` registry-derived prompt routing
- subagent default isolation
- default build and Mentle feature CI coverage
- published `memtle 0.1.2` package source validation

Review outcome:

- Sprint 5 is accepted for Sprint 6 entry.
- Targeted default-lane and Mentle feature-lane verification passed.
- Full workspace `just test` remains blocked by an unrelated provider test export
  issue around `agent_diva_providers::ollama`.

### Sprint 7 Mentle Tool Selection and GUI Controls

Sprint 7 is a planned enhancement Sprint documented in
[25-s7-a1-mentle-tool-selection-and-gui-controls.md](./25-s7-a1-mentle-tool-selection-and-gui-controls.md).

It is intentionally not part of the Sprint 1 through Sprint 6 production
integration gate. Sprint 7 may start only after Sprint 6 has produced an
accepted RC and handoff package.

The enhancement scope covers:

- assembly-level filtering of dynamic `memtle_*` tools
- persisted Mentle settings such as `mentle.enabled`, `mentle.mode`, and
  `allowed_tools`
- optional modes including `off`, `read_only`, `full`, and `custom`
- prompt exposure derived only from the post-filter registry
- a new `agent-diva-gui` General Settings section for enabling Mentle and
  selecting active tools
- manual GUI smoke tests for configuration switching and runtime rebuild
  behavior

Sprint 7 must preserve these existing invariants:

- `memtle_status` remains the prompt-routing activation anchor
- `build_agent_tools(...)` remains the shared registry assembly path
- `with_toolset()` remains registry-driven
- subagents still default to Mentle disabled
- published `memtle` package sourcing remains unchanged

### Sprint 3 Entry Criteria

- S2-A8 review package accepted.
- Verification results are recorded for the default build, Mentle build, provider tests, and agent Mentle tests.
- Sprint 3 implementation agrees to consume only the S2-A8 baseline and the Sprint 3 A1-A8 records unless a separate architecture review changes them.

### Sprint 4 Entry Criteria

- Sprint 3 review package accepted.
- Runtime ownership and active/inactive prompt-routing rules are frozen.
- Minimum verification expectations are documented, including blocked-environment reporting.
- Sprint 4 work agrees to consume the Sprint 3 review package rather than reopening adapter/runtime baselines ad hoc.

### Sprint 5 Entry Criteria

- Sprint 4 review package accepted.
- Prompt/tool consistency is frozen around the `memtle_status` activation anchor.
- Failure-mode work agrees to validate downgrade behavior rather than add new Mentle capabilities.
- Default-lane and Mentle-lane verification gaps are tracked in the Sprint 5 failure validation matrix.
- Published `memtle 0.1.2` package sourcing remains a non-negotiable CI gate.

### Sprint 6 Entry Criteria

- Sprint 5 review package accepted.
- Default-lane Sprint 5 regressions pass locally and in CI.
- Mentle feature lane passes with Rust 1.88+ and the native toolchain available.
- Residual `just test` failure is tracked as unrelated to the Mentle integration
  baseline unless Sprint 6 decides to make full workspace tests an RC blocker.
- Sprint 6 work agrees to package release-candidate evidence rather than reopen
  Sprint 7 GUI/tool-selection scope.

### Sprint 7 Entry Criteria

- Sprint 6 RC and handoff package accepted.
- Production-readiness gates from Sprint 1 through Sprint 6 remain green or have explicit accepted exceptions.
- The common `memtle_*` tools for the read-only preset are identified from the completed integration baseline.
- GUI settings persistence ownership is confirmed.
- The team agrees Sprint 7 is an enhancement Sprint and does not reopen the RC baseline unless a separate change-control review accepts the risk.

## 5. Activity Dependencies

| ID | Activity | Predecessor | Parallel With | Risk |
|---|---|---|---|---|
| A1 | Define dependency, package source, and feature gate path | None | A2 | Default build contamination or local path coupling |
| A2 | Define provider contract | None | A1 | Provider shape mismatch with runtime assembly |
| A3 | Implement `HybridMemoryProvider` | A1, A2 | A4 | Blocking DB calls in prompt path |
| A4 | Implement `MemtleToolkitTool` | A1 | A3 | Tool schema mismatch |
| A5 | Implement `MentleRuntime` | A3, A4 | None | Runtime cannot own reusable custom tools |
| A6 | Implement initial AgentLoop assembly | A5 | A7 | Divergent assembly paths |
| A7 | Preserve tools in cron rebuild | A6 | A8 | Background jobs lose memory tools |
| A8 | Implement context prompt routing | A3, A5 | A7 | Prompt advertises unavailable tools |
| A9 | Implement `with_toolset()` safety behavior | A4, A8 | A10 | External registry mismatch |
| A10 | Implement subagent isolation | A1 | A9 | Subagent pollutes long-term memory |
| A11 | Add failure and regression tests | A7, A8, A9, A10 | A12 | Production gaps not covered |
| A12 | Prepare RC handoff | A11 | None | Merge readiness unclear |
| A13 | Define Mentle tool-selection config | A12 | A14 | Config shape diverges between CLI/service/GUI |
| A14 | Define GUI General Settings UX | A12 | A13 | GUI presents theoretical tools or stale metadata |
| A15 | Filter `memtle_*` tools during runtime assembly | A13 | A16 | Prompt advertises removed tools |
| A16 | Persist GUI settings and trigger runtime rebuild | A14, A15 | A17 | Saved state does not affect active registry |
| A17 | Add prompt/tool subset regression tests | A15 | A16, A18 | Tool subsets bypass existing activation anchor |
| A18 | Prepare Sprint 7 verification and handoff package | A16, A17 | None | Enhancement ships without GUI smoke evidence |

## 6. Gantt

```text
Timeline:      W1        W2        W3        W4        W5        W6        W7
Sprint:        S1        S2        S3        S4        S5        S6        S7

Feature/CI     ######    ##        ##        ##        ####      ####
Core Provider            ######    ##                  ####
Tool Adapter                       ######    ##        ##
Runtime Helper                     ######    ##
AgentLoop Assembly                           ######
Context Routing                              ######    ##
Cron Rebuild                                 ######    ##
with_toolset                                 ######    ##
Subagent Isolation                           ###
Failure Testing                                        ######    ##
Release Docs                                             ##      ######
Tool Selection                                                            ######
GUI Controls                                                              ######
Subset QA                                                                 ####
```

## 7. Kanban

### Backlog

| Card | Title | Owner | Target Sprint |
|---|---|---|---|
| K-17 | Plan release candidate checklist | A-DOC | S6 |
| K-36 | Freeze Sprint 7 Mentle tool-selection scope | A-ARCH | S7 |
| K-37 | Define persisted Mentle tool-selection config | A-CORE | S7 |
| K-38 | Implement assembly-level `memtle_*` filtering | A-LOOP | S7 |
| K-39 | Add prompt/tool synchronization coverage for selected tool subsets | A-QA | S7 |
| K-40 | Add Mentle tools section to `agent-diva-gui` General Settings | A-GUI | S7 |
| K-41 | Persist GUI tool-selection settings and trigger runtime rebuild | A-GUI | S7 |
| K-42 | Run manual GUI smoke for off/read-only/full/custom modes | A-QA | S7 |
| K-43 | Prepare Sprint 7 review and handoff package | A-DOC | S7 |

### Ready

| Card | Title | Owner |
|---|---|---|

### In Progress

| Card | Title | Owner |
|---|---|---|
| K-26 | Use S3-A1 adapter freeze for Sprint 3 implementation | A-MEM |

### Review

| Card | Title | Owner |
|---|---|---|
| K-05 | Plan `HybridMemoryProvider` skeleton | A-CORE |
| K-06 | Plan cached Palace snapshot | A-CORE |
| K-07 | Plan async Mentle prefetch | A-CORE |
| K-08 | Plan `sync_turn()` Mentle write path | A-CORE |
| K-18 | Prepare Sprint 2 review package and Sprint 3 entry criteria | A-ARCH |

### Done

| Card | Title | Owner |
|---|---|---|
| K-00 | Research audit and production-readiness planning input | A-ARCH |
| K-01 | Define workspace optional `memtle` dependency path | A-DEVOPS |
| K-22 | Freeze rule: use Cargo package, not local `mentle/` path dependency | A-ARCH |
| K-02 | Define `agent-diva-core/mentle` feature gate | A-CORE |
| K-03 | Define `agent-diva-agent/mentle` feature propagation | A-LOOP |
| K-04 | Define default and Mentle CI matrix | A-DEVOPS |
| K-19 | Complete Sprint 1 review, acceptance note, and risk refresh | A-DOC |
| K-20 | Freeze published `memtle 0.1.2` source, version, and upgrade policy | A-DEVOPS |
| K-21 | Record Cargo package source constraint in technical design docs | A-DOC |
| K-23 | Record published `memtle` feature and toolchain constraints for Sprint 2 | A-DOC |
| K-25 | Create S2-A8 Sprint 2 review package and Sprint 3 interface baseline | A-DOC |
| K-24 | Review S2-A8 package and approve Sprint 3 interface baseline | A-ARCH |
| K-09 | Plan `MentleToolkitTool` adapter | A-MEM |
| K-27 | Freeze `MemtleToolkitTool` adapter interface for S3-A1 | A-DOC |
| K-10 | Plan dynamic tool registration | A-MEM |
| K-28 | Freeze toolkit error mapping for S3-A3 | A-MEM |
| K-11 | Plan `MentleRuntime` helper | A-LOOP |
| K-29 | Define Sprint 3 QA baseline for Sprint 4 entry | A-QA |
| K-30 | Prepare Sprint 3 review package and Sprint 4 entry baseline | A-DOC |
| K-12 | Harden unified `build_agent_tools` helper | A-LOOP |
| K-13 | Harden cron rebuild custom tool preservation | A-LOOP |
| K-14 | Harden `with_toolset()` safety behavior | A-LOOP |
| K-15 | Harden subagent default Mentle isolation | A-LOOP |
| K-31 | Review S4 adapter/runtime compatibility against S3 conventions | A-MEM |
| K-32 | Freeze Sprint 4 regression baseline for advanced assembly paths | A-QA |
| K-33 | Record Mentle feature build environment and Windows LLVM PATH prerequisite | A-DEVOPS |
| K-34 | Consolidate Sprint 4 summary, verification, acceptance, and release notes | A-DOC |
| K-35 | Prepare Sprint 4 architecture sign-off and review package | A-ARCH |
| K-16 | Execute Sprint 5 failure validation matrix | A-QA |

## 8. Acceptance Gates

Future implementation cannot be called production-ready until all of these are true:

- Default build does not enable Mentle or pull `memtle`.
- Mentle is sourced from a published Cargo package rather than a local `mentle/` path dependency.
- Mentle build is explicit and passes on Rust 1.88+.
- Sprint 2 and Sprint 3 design notes cite only published-crate-confirmed Mentle surfaces.
- Sprint 3 runtime/tool work consumes the S2-A8 interface baseline.
- `MemtleToolkit::open()` success is required before Mentle prompt and tools are enabled.
- `MemtleToolkit::open()` failure downgrades to Markdown memory without tool hallucination.
- `system_prompt_block()` does not block on SQLite.
- Tool registration is dynamic from `tool_definitions()`.
- Initial tool assembly and cron rebuild reuse the same Mentle custom tool set.
- `with_toolset()` does not create prompt/tool mismatch.
- Subagents default to Mentle disabled.
- CI covers default build, Mentle build, provider tests, assembly tests, cron rebuild tests, `with_toolset()` tests, and subagent isolation tests.
- Sprint 7 enhancements do not block Sprint 1-6 production readiness; they require an accepted Sprint 6 RC before implementation starts.
- When Sprint 7 is implemented, configured Mentle tool subsets must preserve prompt/tool consistency and keep `memtle_status` as the activation anchor.
- `agent-diva-gui` must persist Mentle tool-selection settings and prove the selected mode affects the active registry through a documented smoke test.

## 9. Definition of Done

A Sprint is done only when:

- Its deliverables are reviewed by the responsible owner.
- Interface assumptions are recorded.
- Tests or explicit verification tasks are attached.
- New risks are added to the register.
- No undocumented prompt/tool mismatch remains open.
- Any dependency source rule changes are reflected in both the project management doc and technical design docs.
