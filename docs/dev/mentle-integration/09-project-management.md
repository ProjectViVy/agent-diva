# Mentle x Agent-Diva Project Management

## 1. Objective

This document tracks the agile delivery plan for production-grade Mentle integration into Agent-Diva. The target outcome is:

- Keep `MEMORY.md` as L0/L1 Compass memory.
- Add Mentle/Memtle as L2 Palace memory.
- Inject `memtle_*` tools dynamically.
- Route memory lifecycle through `HybridMemoryProvider`.
- Keep startup, cron rebuild, `with_toolset()`, subagent spawning, feature gates, and CI in a production-safe state.

Current delivery state:

- Sprint 1 is complete and has passed review.
- Sprint 2 is ready to start.
- No Rust implementation has been merged yet for this initiative.

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
| 1.0 | Dependency and feature foundation | Optional `mentle` feature path, default build isolation, Rust 1.88+ Mentle build path | A-DEVOPS | Build strategy frozen |
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
| Sprint 2 | Core hybrid memory provider | Ready | `HybridMemoryProvider` contract and cached snapshot path |
| Sprint 3 | Tool adapter and runtime helper | Planned | Dynamic `memtle_*` tools and `MentleRuntime` |
| Sprint 4 | AgentLoop, cron, `with_toolset()`, subagent | Planned | Assembly closure and memory-safety behavior |
| Sprint 5 | Failure modes and CI hardening | Planned | Regression and downgrade confidence |
| Sprint 6 | RC and handoff | Planned | Release candidate package |

### Sprint 1 Review

Completed activities:

- Defined the workspace-level Mentle dependency strategy.
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
- The team agrees that no prompt path may block on async DB calls.

## 5. Activity Dependencies

| ID | Activity | Predecessor | Parallel With | Risk |
|---|---|---|---|---|
| A1 | Define dependency and feature gate path | None | A2 | Default build contamination |
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
| K-09 | Plan `MentleToolkitTool` adapter | A-MEM | S3 |
| K-10 | Plan dynamic tool registration | A-MEM | S3 |
| K-11 | Plan `MentleRuntime` helper | A-LOOP | S3 |
| K-12 | Plan unified `build_agent_tools` helper | A-LOOP | S4 |
| K-13 | Plan cron rebuild custom tool preservation | A-LOOP | S4 |
| K-14 | Plan `with_toolset()` safety behavior | A-LOOP | S4 |
| K-15 | Plan subagent default Mentle isolation | A-LOOP | S4 |
| K-16 | Plan failure injection tests | A-QA | S5 |
| K-17 | Plan release candidate checklist | A-DOC | S6 |

### Ready

| Card | Title | Owner |
|---|---|---|
| K-05 | Plan `HybridMemoryProvider` skeleton | A-CORE |
| K-06 | Plan cached Palace snapshot | A-CORE |
| K-07 | Plan async Mentle prefetch | A-CORE |
| K-08 | Plan `sync_turn()` Mentle write path | A-CORE |
| K-18 | Prepare Sprint 2 kickoff package and entry criteria | A-ARCH |

### In Progress

None. Sprint 2 has not started yet.

### Review

None.

### Done

| Card | Title | Owner |
|---|---|---|
| K-00 | Research audit and production-readiness planning input | A-ARCH |
| K-01 | Define workspace optional `memtle` dependency path | A-DEVOPS |
| K-02 | Define `agent-diva-core/mentle` feature gate | A-CORE |
| K-03 | Define `agent-diva-agent/mentle` feature propagation | A-LOOP |
| K-04 | Define default and Mentle CI matrix | A-DEVOPS |
| K-19 | Complete Sprint 1 review, acceptance note, and risk refresh | A-DOC |

## 8. Acceptance Gates

Future implementation cannot be called production-ready until all of these are true:

- Default build does not enable Mentle or pull `memtle`.
- Mentle build is explicit and passes on Rust 1.88+.
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
