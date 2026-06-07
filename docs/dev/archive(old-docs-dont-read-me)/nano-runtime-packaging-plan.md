# Nano Runtime And Packaging Plan

## 1. Purpose

This document defines a current-state, implementation-oriented plan for the `agent-diva-nano` line after the recent decoupling and cleanup work.

It answers two questions:

1. Is the nano line now materially more feasible than before?
2. Is the target direction of "publish reusable main-project crates to crates.io, while `agent-diva-nano` becomes a lightweight standalone distribution shell" structurally sound?

Short answer:

- **Yes**, the direction is now materially more feasible than before.
- **No**, the current state should **not** yet be treated as the final stable architecture.
- The correct next move is **not** to collapse nano back into the main workspace, but to **finish boundary hardening** so the lightweight line consumes a stable reusable runtime surface instead of directly depending on a wide internal crate closure.

## 2. Current State Summary

### 2.1 What is already better than before

- `agent-diva-nano` is no longer a root workspace member.
- The main product line is again centered on `agent-diva-cli` + `agent-diva-manager`.
- `agent-diva-nano` no longer directly depends on `agent-diva-manager`.
- The nano source tree now owns its local runtime/control-plane modules instead of re-exporting manager internals through cross-crate source inclusion.

This means the project has already crossed the most important conceptual threshold:

- **nano is now a separate product line candidate**
- instead of **a feature branch hidden inside the formal product graph**

### 2.2 What is still unfinished

The nano line is still not a cleanly packaged lightweight starter in the architectural sense.

It still directly depends on:

1. `agent-diva-core`
2. `agent-diva-agent`
3. `agent-diva-providers`
4. `agent-diva-channels`
5. `agent-diva-tools`

That is acceptable as an intermediate state, but weak as a long-term product boundary.

### 2.3 Immediate correctness issue in the current tree

At the time of writing, `agent-diva-nano` in `.workspace/nano-workspace/agent-diva-nano` still uses relative dependency paths like:

- `../../agent-diva-core`
- `../../agent-diva-agent`

From the current directory layout, those paths resolve into `.workspace/agent-diva-*` rather than the repository root. That means the current nested workspace layout and the current `Cargo.toml` do not form a valid build graph together.

This is not a philosophical issue. It is a concrete packaging and build-chain defect and must be fixed before any "nano is ready" claim.

## 3. Design Judgment

### 3.1 Is nano now more feasible?

**Yes.**

Compared with the previous state, the project now has:

- a clearer product split
- a clearer mainline runtime
- a less dangerous dependency relation
- a better foundation for future extraction

The direction is therefore **feasible enough to continue investing in**.

### 3.2 Is the current design already reasonable as a stable end state?

**Not yet.**

The current shape still has three structural weaknesses:

1. `agent-diva-nano` depends on a broad internal closure rather than a narrow stable runtime surface.
2. `agent-diva-nano` and `agent-diva-manager` still carry overlapping control-plane logic and therefore risk long-term drift.
3. The docs/build/release narrative is inconsistent with the actual filesystem layout.

So the current state is best understood as:

- **post-decoupling transitional architecture**

not:

- **completed lightweight product architecture**

## 4. Architectural Problems That Still Need To Be Solved

### 4.1 Wide dependency surface

Today nano directly imports internal capabilities from multiple crates:

- config schema
- bus/session/cron types
- provider catalog and provider access
- channel manager
- tool-side MCP probing
- agent runtime control and skills loading

This creates a broad coupling surface. The practical consequence is:

- any internal reshaping in `agent-diva-agent`, `agent-diva-channels`, `agent-diva-providers`, or `agent-diva-tools`
- can break nano even when no nano-facing product contract changed

This is the central reason the current shape is not yet stable enough.

### 4.2 Control-plane duplication risk

The present split between `agent-diva-manager` and `agent-diva-nano` is not a thin-shell split over a shared library core. It is closer to:

- one formal manager runtime
- one copied-and-trimmed nano runtime/control-plane

This is acceptable during extraction prep, but poor as a long-lived maintenance strategy. Once both sides evolve independently, the project will pay repeated costs in:

- HTTP route behavior drift
- config update drift
- skill/MCP admin drift
- cron/session admin drift
- inconsistent bug fixes

### 4.3 "Lightweight" is currently product-level, not dependency-level

Nano is called a lightweight line, but the current crate closure still pulls in full channel/provider/tool capability through the same major internal crates.

That means the current lightweight property is mainly:

- lighter product positioning
- lighter packaging target
- lighter standalone shell

and not yet:

- significantly smaller runtime closure
- significantly narrower compile-time API surface
- significantly stronger modular isolation

This is acceptable if intentionally documented, but weak if presented as a fully realized lightweight runtime architecture.

### 4.4 Documentation and release narrative drift

There are still references to:

- `external/agent-diva-nano/`
- `cd external`
- old extraction links or path assumptions

This creates operational confusion:

- contributors may run commands in the wrong directory
- packaging instructions become unreliable
- future refactors are made against stale assumptions

This must be treated as an architecture hygiene issue, not just a docs nit.

## 5. Recommended Target Architecture

The target architecture should preserve the current product split, but reduce cross-line coupling by introducing a **shared internal library layer for runtime assembly and control-plane behavior**.

Recommended steady-state shape:

1. `agent-diva-core`
2. `agent-diva-runtime`
3. `agent-diva-control-plane`
4. `agent-diva-manager`
5. `agent-diva-nano`
6. `agent-diva-cli`

### 5.1 Role of each crate

#### `agent-diva-core`

Keep only cross-cutting stable domain primitives here:

- config schema
- bus contracts
- session/cron domain types
- shared IDs and core traits

This crate should remain the most stable and least product-opinionated layer.

#### `agent-diva-runtime`

New shared library crate.

Purpose:

- assemble providers/channels/tools/agent loop into a reusable runtime
- expose a stable runtime bootstrap surface
- centralize lifecycle orchestration

Typical responsibilities:

- runtime bootstrap
- provider/channel/tool registry assembly
- shutdown handling
- session runtime control bridge
- cron-to-agent dispatch bridge

This crate should become the main reusable execution core used by both manager and nano.

#### `agent-diva-control-plane`

New shared library crate.

Purpose:

- hold reusable HTTP/admin/control-plane behavior
- remove route and admin logic duplication between manager and nano

Typical responsibilities:

- control-plane state types
- shared API command enums
- shared handlers
- skill/MCP/session/cron admin orchestration
- config update DTOs
- streaming/event endpoint behavior

#### `agent-diva-manager`

Formal manager product shell.

Responsibilities:

- manager-specific defaults
- manager-specific branding or packaging semantics
- mainline gateway composition
- formal release-facing binary/library shell

This crate should become thin.

#### `agent-diva-nano`

Lightweight standalone shell.

Responsibilities:

- starter-oriented defaults
- lightweight packaging layout
- simplified operator experience
- optional limited capability profile

This crate should also become thin.

#### `agent-diva-cli`

Formal user-facing CLI product.

Responsibilities:

- mainline command UX
- mainline distribution entry
- manager-backed local gateway path

The CLI should not be re-coupled to nano.

### 5.2 Target dependency DAG

Recommended DAG:

```text
agent-diva-core
    ^
    |
agent-diva-providers   agent-diva-tools   agent-diva-channels
        ^                    ^                 ^
        |                    |                 |
        +-------- agent-diva-runtime ---------+
                           ^
                           |
                agent-diva-control-plane
                    ^                ^
                    |                |
           agent-diva-manager   agent-diva-nano
                    ^
                    |
               agent-diva-cli
```

Important properties:

- `agent-diva-manager` and `agent-diva-nano` stop depending on the wide internal world directly.
- both depend on the same runtime/control-plane contracts.
- product-line differences move from copied code to thin-shell composition.

## 6. Packaging Strategy Judgment

### 6.1 Is "publish reusable crates to crates.io + nano as a standalone shell" reasonable?

**Yes.**

This is the most reasonable long-term distribution strategy for the current repository direction.

It gives:

- reusable internal crates
- a clean starter/template path
- the ability to evolve mainline and lightweight lines at different product speeds
- a clear separation between framework reuse and end-product packaging

### 6.2 Conditions for this strategy to be truly healthy

This strategy is healthy only if all of the following are true:

1. Shared crates expose stable, intentional APIs.
2. Product shells do not depend on broad internals directly.
3. Publishing order is explicit and testable.
4. Nano can build either:
   - inside the monorepo staging area, or
   - as a fully extracted repository,
   without hidden workspace assumptions.

If these are not met, then publishing to crates.io merely moves coupling from path edges to versioned breakage.

### 6.3 What should be published

Recommended publication candidates:

- `agent-diva-core`
- `agent-diva-providers`
- `agent-diva-tools`
- `agent-diva-agent` only if its API is intentionally consumable
- `agent-diva-channels` only if its API is intentionally consumable
- new `agent-diva-runtime`
- new `agent-diva-control-plane`

Publication should follow architectural readiness, not just current existence.

If a crate has a large unstable internal surface, it should either:

- be narrowed before publishing
- or remain internal until the surface is intentional

## 7. Recommended Capability Boundary For Nano

Nano does **not** need to be artificially tiny to be valid. But it should be intentionally bounded.

Recommended boundary for nano v1 steady state:

- single local gateway runtime
- local HTTP control plane
- shared config schema consumption
- shared provider/channel/tool runtime composition
- starter-oriented operator UX

Nano should avoid becoming:

- a second formal manager product line
- a silent fork of manager
- a "copy of everything with fewer claims"

### 7.1 Optional future narrowing

After the shared runtime/control-plane layer is established, the project can optionally make nano lighter through features such as:

- smaller default channel set
- smaller default provider set
- smaller default tool set
- starter-mode feature flags
- runtime profiles

That work should happen **after** architecture stabilization, not before.

## 8. Phased Migration Plan

### Phase 0: Correctness And Narrative Repair

Goal:

- make the current transitional layout truthful and buildable

Tasks:

1. Fix `agent-diva-nano` dependency paths relative to `.workspace/nano-workspace/agent-diva-nano`.
2. Update all stale references from `external/agent-diva-nano` to the current staging path.
3. Update build instructions, extraction notes, and release helper references.
4. Re-run targeted build/metadata validation for nano from its actual workspace root.

Exit criteria:

- `cargo metadata` succeeds in `.workspace/nano-workspace`
- `cargo check -p agent-diva-nano` succeeds from `.workspace/nano-workspace`
- docs no longer describe a non-existent layout

### Phase 1: Runtime Surface Extraction

Goal:

- reduce nano's direct dependency on broad internal crate APIs

Tasks:

1. Identify the actual runtime assembly API used by both manager and nano.
2. Move reusable bootstrap/lifecycle/orchestration logic into a new `agent-diva-runtime` crate.
3. Keep manager and nano as consumers of the same runtime bootstrap API.
4. Remove duplicated runtime lifecycle code from product shells.

Exit criteria:

- manager and nano both depend on `agent-diva-runtime`
- manager and nano no longer own divergent runtime bootstrap logic

### Phase 2: Control-Plane Surface Extraction

Goal:

- stop duplicating control-plane logic across manager and nano

Tasks:

1. Extract shared state types, commands, and DTOs into `agent-diva-control-plane`.
2. Extract shared handlers for config, session, cron, skill, MCP, and event APIs.
3. Keep only shell-specific wiring in manager and nano.
4. Ensure route behavior remains semantically aligned between both products.

Exit criteria:

- manager and nano use shared control-plane library code
- API drift risk is materially reduced

### Phase 3: Publication Boundary Hardening

Goal:

- make the crates.io strategy intentional and sustainable

Tasks:

1. Decide which crates are public-stable and which are still internal.
2. Minimize unstable public API surfaces before publication.
3. Define the topo publish order and enforce it in tooling.
4. Verify `cargo package` / `cargo publish --dry-run` on all publish candidates.

Exit criteria:

- publish order is documented and automated
- public crates package without hidden workspace assumptions

### Phase 4: Nano Repository Extraction Or Monorepo Staging Finalization

Goal:

- choose one operational model and make it real

Two acceptable models:

#### Option A: Keep nano staged inside the monorepo

Use `.workspace/nano-workspace` as the long-term staging location, but ensure:

- it builds correctly
- it consumes only stable public/internal surfaces
- its docs match reality

#### Option B: Extract nano to its own repository

Move nano once:

- runtime/control-plane boundaries are stable
- published crates are available or git dependency policy is explicit

Recommended default:

- **do not extract immediately**
- **finish Phases 0 to 3 first**

This avoids freezing a bad API boundary into a second repository too early.

## 9. Validation Strategy

Each phase should be validated with both build closure checks and product behavior checks.

### 9.1 Minimum validation for Phase 0

- `cargo metadata --format-version 1`
- `cargo check -p agent-diva-nano`
- `cargo test -p agent-diva-nano`
- manual doc path audit

### 9.2 Minimum validation for Phase 1 and Phase 2

- `just fmt-check`
- `just check`
- `just test`
- targeted crate checks for new shared crates
- manager smoke path
- nano smoke path

### 9.3 Minimum smoke expectations

Manager smoke:

- start local gateway path through the mainline product route
- verify config/session/event endpoints remain healthy

Nano smoke:

- build and run the standalone nano local gateway path
- verify the same critical control-plane endpoints

## 10. Main Risks

### 10.1 Extracting too early

If nano is moved into a separate repository before stable shared boundaries exist, the project will turn current internal churn into cross-repository release pain.

### 10.2 Publishing unstable internals as if they are stable APIs

Crates.io publication is not architecture. Publishing broad unstable internals too early will increase maintenance burden and version coordination cost.

### 10.3 Keeping duplicated manager/nano logic for too long

This is likely the highest medium-term maintenance risk. The longer shared behavior exists in copied form, the harder convergence becomes.

### 10.4 Chasing lightweight claims too early

If the project optimizes for "smaller" before it optimizes for "cleaner boundaries", it will likely create special cases and feature fragmentation.

## 11. Recommended Final Position

The project should adopt the following stance:

- `agent-diva-cli` + `agent-diva-manager` remain the formal main product line.
- `agent-diva-nano` remains the lightweight starter/template line.
- The lightweight line should remain separate from the formal mainline product graph.
- The next step is **shared boundary extraction**, not reintegration and not premature external split.
- The crates.io strategy is valid, but only after runtime/control-plane surfaces are intentionally stabilized.

## 12. Immediate Action Checklist

Recommended next concrete actions, in order:

1. Fix the broken nano relative dependency paths.
2. Repair all stale `external/` and broken extraction doc references.
3. Add a targeted build check for `.workspace/nano-workspace`.
4. Extract a new shared `agent-diva-runtime` crate.
5. Extract a new shared `agent-diva-control-plane` crate.
6. Reduce manager and nano to thin product shells over those shared crates.
7. Re-evaluate which crates are truly ready for crates.io publication.
8. Only then decide whether nano should remain monorepo-staged or become a separate repository.

## 13. Decision

Decision for the current stage:

- **Continue the nano line**
- **Do not collapse it back into the main workspace**
- **Do not treat the current structure as final**
- **Complete boundary hardening before publication/extraction**

That is the most defensible path for the current repository state.
