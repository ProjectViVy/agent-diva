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
- Sprint 3 must consume the interface baseline in [12-s2-a8-sprint2-review-and-s3-interface-baseline.md](./12-s2-a8-sprint2-review-and-s3-interface-baseline.md), the adapter freeze in [13-s3-a1-memtle-toolkit-tool-interface.md](./13-s3-a1-memtle-toolkit-tool-interface.md), the dynamic registration model in [14-s3-a2-dynamic-tool-registration-model.md](./14-s3-a2-dynamic-tool-registration-model.md), and the toolkit error mapping in [15-s3-a3-toolkit-error-mapping.md](./15-s3-a3-toolkit-error-mapping.md).

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

## 4. Sprint Roadmap

Assumption: one-week Sprints.

| Sprint | Theme | Status | Main Outcome |
|---|---|---|---|
| Sprint 1 | Build foundation and feature gates | Completed | Default build isolation and Mentle feature boundary defined |
| Sprint 2 | Core hybrid memory provider | Completed | `HybridMemoryProvider` contract, cached snapshot path, and published-package implementation constraints packaged for Sprint 3 |
| Sprint 3 | Tool adapter and runtime helper | In progress | Dynamic `memtle_*` tools and `MentleRuntime` consume the S2-A8 baseline and S3-A1 adapter freeze |
| Sprint 4 | AgentLoop, cron, `with_toolset()`, subagent | Planned | Assembly closure and memory-safety behavior |
| Sprint 5 | Failure modes and CI hardening | Planned | Regression and downgrade confidence |
| Sprint 6 | RC and handoff | Planned | Release candidate package |

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

### Sprint 3 Entry Criteria

- S2-A8 review package accepted.
- Verification results are recorded for the default build, Mentle build, provider tests, and agent Mentle tests.
- Sprint 3 implementation agrees to consume only the S2-A8 baseline, S3-A1 adapter freeze, S3-A2 dynamic registration model, and S3-A3 toolkit error mapping unless a separate architecture review changes them.

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

## 6. Gantt

```text
Timeline:      W1        W2        W3        W4        W5        W6
Sprint:        S1        S2        S3        S4        S5        S6

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
```

## 7. Kanban

### Backlog

| Card | Title | Owner | Target Sprint |
|---|---|---|---|
| K-12 | Plan unified `build_agent_tools` helper | A-LOOP | S4 |
| K-13 | Plan cron rebuild custom tool preservation | A-LOOP | S4 |
| K-14 | Plan `with_toolset()` safety behavior | A-LOOP | S4 |
| K-15 | Plan subagent default Mentle isolation | A-LOOP | S4 |
| K-16 | Plan failure injection tests | A-QA | S5 |
| K-17 | Plan release candidate checklist | A-DOC | S6 |

### Ready

| Card | Title | Owner |
|---|---|---|
| K-11 | Plan `MentleRuntime` helper | A-LOOP |

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

## 9. Definition of Done

A Sprint is done only when:

- Its deliverables are reviewed by the responsible owner.
- Interface assumptions are recorded.
- Tests or explicit verification tasks are attached.
- New risks are added to the register.
- No undocumented prompt/tool mismatch remains open.
- Any dependency source rule changes are reflected in both the project management doc and technical design docs.
