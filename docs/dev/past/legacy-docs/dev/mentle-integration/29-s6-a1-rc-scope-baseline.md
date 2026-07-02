# Sprint 6 A1: RC Scope Baseline and Release Boundary

## Purpose

This record starts Sprint 6 by freezing the release-candidate scope for the
Mentle x Agent-Diva Sprint 1 through Sprint 6 mainline.

Sprint 6 is an RC and handoff Sprint. It packages the completed integration
baseline, final acceptance evidence, release notes, rollback notes, and merge
readiness. It must not add new main features, reopen the Sprint 7 GUI/tool
selection lane, or broaden the runtime memory model without a separate
architecture review.

## Freeze Statement

The RC scope is frozen as of S6-A1.

No new main Mentle integration features should be added during Sprint 6. S6
work may update evidence, documentation, checklists, release notes, rollback
guidance, and narrowly scoped fixes for true RC blockers. Any feature-like work
that changes tool selection, GUI controls, runtime hot-refresh, dependency
source policy, subagent inheritance, or memory routing semantics is out of RC
scope and belongs to a later Sprint after explicit acceptance.

## Baseline Consumed

S6-A1 consumes the completed Sprint 1 through Sprint 5 chain:

- [09-project-management.md](./09-project-management.md)
- [18-s3-a8-sprint3-review-package.md](./18-s3-a8-sprint3-review-package.md)
- [24-s4-a12-sprint4-review-package.md](./24-s4-a12-sprint4-review-package.md)
- [26-s5-a1-failure-validation-matrix.md](./26-s5-a1-failure-validation-matrix.md)
- [27-s5-a2-a6-failure-and-ci-hardening.md](./27-s5-a2-a6-failure-and-ci-hardening.md)
- [28-s5-a7-sprint5-review-package.md](./28-s5-a7-sprint5-review-package.md)

It also explicitly references Sprint 7 planning only to keep the post-RC
enhancement boundary closed:

- [25-s7-a1-mentle-tool-selection-and-gui-controls.md](./25-s7-a1-mentle-tool-selection-and-gui-controls.md)

## RC Scope Baseline

### In Scope for RC

| Area | RC baseline | Evidence anchor |
|---|---|---|
| Package source | Agent-Diva consumes published `memtle 0.1.2` with `default-features = false`; no local path, git, or `[patch.crates-io]` override is allowed | `Cargo.toml`, `Cargo.lock`, `scripts/ci/verify_mentle_package_policy.py`, S5-F16 |
| Feature isolation | Default Agent-Diva builds remain Mentle-free unless the explicit `mentle` Cargo feature is enabled | `just sprint5-default-check`, CI `rust-check` |
| Memory model | `MEMORY.md` and `HISTORY.md` remain L0/L1 Compass memory; Mentle is L2 Palace memory when runtime activation succeeds | S2/S3 baseline, `ContextBuilder` prompt routing |
| Prompt injection | `system_prompt_block()` stays provider-driven and must not block on fresh SQLite work in the prompt path | S2/S5 memory provider records |
| Runtime assembly | `MentleRuntime` owns toolkit, hybrid provider, custom tools, and construction-time active state | `agent-diva-agent/src/mentle_runtime.rs`, S3/S4 review packages |
| Dynamic tools | `MemtleToolkit::tool_definitions()` is the only dynamic schema source; adapter consumes `name`, `description`, and object `inputSchema` | S3-A1/S3-A2/S4-A8 |
| Tool execution | `MemtleToolkit::call_json(name, args).await` remains the only generic adapter execution path | S3-A1/S4-A8 |
| Activation anchor | Mentle prompt routing activates only when the assembled registry/runtime contains `memtle_status` | S4-A9, S5-F03, S5-F11, S5-F12 |
| Rebuild paths | Initial assembly, startup cron rebuild, and later default-tool rebuild preserve runtime custom tools through `build_agent_tools(...)` | S4-A9, S5-F10 |
| `with_toolset()` | External registries drive prompt activation; `with_toolset()` does not synthesize a Mentle runtime or hybrid provider | S4-A9, S5-F11, S5-F12 |
| Subagents | Subagent config, registry, and prompt default to Mentle disabled and do not inherit parent Mentle custom tools | S4-A9, S5-F13 |
| Failure downgrade | Startup/open failure disables Mentle and falls back to Markdown memory without advertising `memtle_*` tools | S5-F01, S5-F02 |
| Recall/write failures | Recall failure is non-fatal; diary write failure keeps file-backed history authoritative when file writes succeed | S5-F07, S5-F08, S5-F09 |
| CI/local gates | Default lane, Mentle feature lane, and package policy checks are callable locally and represented in CI | `just sprint5-default-check`, `just mentle-check`, CI `mentle-check` |

### Out of Scope for RC

| Area | Boundary |
|---|---|
| Sprint 7 tool selection | No `off`/`read_only`/`full`/`custom` tool-selection implementation in Sprint 6 |
| GUI controls | No new `agent-diva-gui` Mentle settings panel or dynamic checklist in Sprint 6 |
| Runtime hot-refresh | No live refresh of `MemtleToolkit::tool_definitions()` after runtime construction |
| Dependency source changes | No local `memtle` path/git override and no version upgrade during RC unless a blocker forces a reviewed patch |
| Subagent inheritance | No default inheritance of Mentle tools, Palace memory, or parent custom tools |
| New memory model | No replacement of the L0/L1 Markdown plus L2 Palace split |
| Prompt semantics | No activation based only on config, Cargo feature, or toolkit-open success without `memtle_status` in the actual registry |
| Broad refactors | No large AgentLoop, memory provider, GUI, or CI restructuring unrelated to RC blockers |

## Blocker and Non-Blocker Classification

| Classification | Item | RC interpretation | Required S6 handling |
|---|---|---|---|
| Blocker | Package source policy violation for `memtle` | Would break reproducible release sourcing | Fix before RC; rerun package policy check |
| Blocker | Default build pulls or requires Mentle unexpectedly | Would break feature isolation and non-Mentle users | Fix before RC; rerun default-lane checks |
| Blocker | Prompt advertises L2/Mentle capability without `memtle_status` in the active registry | Would create a user-visible hallucinated capability | Fix before RC; rerun prompt/tool regressions |
| Blocker | Mentle startup failure prevents AgentLoop startup instead of downgrading to Markdown memory | Would break the downgrade contract | Fix before RC; rerun S5-F01/S5-F02 evidence |
| Blocker | Subagents inherit parent Mentle long-term memory by default | Would violate isolation and data-boundary assumptions | Fix before RC; rerun subagent config, registry, and prompt tests |
| Blocker | Mentle feature lane fails on a host with Rust 1.88+ and native prerequisites present | Indicates an integration regression | Fix before RC or record an explicit accepted exception |
| Non-blocker | Full workspace `just test` provider export failure around `agent_diva_providers::ollama` | Already recorded as outside Sprint 5 Mentle scope unless Sprint 6 promotes full workspace tests to RC gate | Track in release notes and RC risk register |
| Non-blocker | Local Windows shell missing `clang-cl.exe` on PATH | Environment setup issue with documented fix | Record as blocked only when prerequisite is absent; otherwise prefix PATH and rerun Mentle checks |
| Non-blocker | `with_toolset()` does not install a `HybridMemoryProvider` | Intentional external-registry boundary | Document in handoff; do not change during RC |
| Non-blocker | Runtime active state is construction-time only | Intentional no-hot-refresh boundary | Keep out of RC; hand off to Sprint 7+ if needed |
| Non-blocker | Sprint 7 GUI/tool-selection work not implemented | Explicit post-RC enhancement | Keep gated behind accepted S6 RC |

S6-A1 found no new code/documentation fact mismatch that requires immediate code
change. The only known potentially release-relevant open item is whether Sprint
6 owners decide to promote full workspace `just test` to an RC blocker despite
the existing provider export issue being outside the Mentle integration
baseline.

## Release Boundary Note

The release candidate may claim the Sprint 1 through Sprint 6 Mentle integration
baseline only for the frozen behavior above:

- published-package Mentle dependency under an explicit feature gate
- Markdown memory continuity with optional L2 Palace memory
- dynamic `memtle_*` tool registration from toolkit definitions
- registry-derived prompt activation anchored on `memtle_status`
- downgrade behavior for startup, recall, write, and invalid-definition failures
- cron/default rebuild, `with_toolset()`, and subagent isolation regressions
- documented local and CI validation lanes

The release candidate must not claim:

- GUI-managed Mentle modes
- per-tool user selection
- runtime hot-refresh of dynamic tool definitions
- default subagent access to Mentle memory
- full workspace test health beyond the explicitly accepted test exception

## S6-A2 Through S6-A8 Execution Baseline

| ID | Owner lane | Work item | Acceptance口径 |
|---|---|---|---|
| S6-A2 | A-QA | Replay and summarize RC verification evidence | Records default lane, package policy, Mentle feature lane, and known `just test` exception with command/result status |
| S6-A3 | A-DEVOPS | Release readiness and environment gate | Confirms Rust/toolchain prerequisites, CI jobs, package policy, and platform notes; no long-running full CI unless explicitly promoted |
| S6-A4 | A-DOC | Handoff package skeleton | Produces summary, verification, acceptance, release, and rollback notes for RC reviewers |
| S6-A5 | A-ARCH/A-QA | Final manual acceptance checklist | Maps user-visible acceptance to the frozen RC scope and excludes Sprint 7 features |
| S6-A6 | A-ARCH/A-DEVOPS | Blocker triage and risk register | Classifies every open item as blocker, accepted exception, or post-RC follow-up |
| S6-A7 | A-DOC | RC release notes and rollback note | Describes shipped boundaries, known non-blockers, and rollback from Mentle feature usage to Markdown memory |
| S6-A8 | A-ARCH | Final review package and Sprint 7 entry gate | Records RC sign-off, merge/release readiness, and explicit gate for Sprint 7 enhancement work |

Suggested model routing for later expert work must avoid Anthropic/OPUS models:

| Work | Complexity | Suggested model |
|---|---|---|
| S6-A2 verification synthesis | Complex | GPT5.5 |
| S6-A3 release/environment gate | Complex | GPT5.5 |
| S6-A4 handoff package drafting | Simple to medium | Gemini 3.1 Pro |
| S6-A5 manual acceptance checklist | Simple to medium | Gemini 3.1 Pro |
| S6-A6 blocker/risk triage | Complex | GPT5.5 |
| S6-A7 release/rollback notes | Simple to medium | Gemini 3.1 Pro |
| S6-A8 final architecture sign-off | Complex | GPT5.5 |

## S6-A1 Validation

S6-A1 is a documentation and scope-freeze task. No Rust source, GUI source, CI
workflow, dependency, or test behavior was changed.

Recommended lightweight validation for this record:

```powershell
git status --short
Test-Path agent-diva/docs/dev/mentle-integration/29-s6-a1-rc-scope-baseline.md
```

Full `just test` is intentionally not part of S6-A1 because this task does not
modify executable behavior, and Sprint 5 already records a pre-existing
non-Mentle provider export issue for that command.
