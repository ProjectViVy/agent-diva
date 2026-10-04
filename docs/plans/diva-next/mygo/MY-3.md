# MY-3: Compare lifecycle cost and record the observation decision — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: after implementation authorization,
> use **superpowers:executing-plans** task by task. Delegation is not selected.
> Steps use checkbox syntax; this planning publication does not start code work.

**Goal:** Compare lifecycle cost and record the observation decision.
**Architecture:** Use an accepted installed Wails baseline and the installed MyGo candidate; preserve historical evidence and make an owner-reviewable recommendation.
**Tech Stack:** Pinned MyGo v0.2.4, isolated Go 1.27.1, public VIVY SDK,
retained Vue/TypeScript and Windows WebView2.
**Spec:** [MY-D1](design.md), revision MY-D1 / 2026-10-04.
**Requirements:** MY-R1, MY-R5, MY-R6.
**Status/dependencies:** [Authoritative index](index.md#stories-and-readiness).
**Baseline:** DIVA 5444795a2d9db31e158c2cf009d64697e6289e50;
VIVY fc559e6b03ce4e65c0099b9745855dccc4fb067e.

## Global Constraints

Planning only is authorized. Preserve Wails delivery, one Runtime/Journal,
sealed Generation, public SDK-only embedding, native sender authority, existing
DTOs and permissions. Root context differs from page context.
Close budget is five seconds; Next batch maximum is 500 and Host queue capacity
is 10,000. Speech limits/namespace are exactly MY-D1 / W3-3.
Proposed source paths do not exist at the inspected baseline.
Integrate accepted predecessor outputs and reconcile source before execution.
Source reads/mock tests do not replace native, provider or owner acceptance.


## Review Focus

Different SDK/frontend/compiler inputs can confound measurements; missing results
must not look like passes; hello-world/native-toolkit numbers cannot represent DIVA;
unpaid upstream patch effort matters; a recommendation must not trigger migration.

## Task 1: Produce a reproducible comparison

**Files:** Modify observations.md; evidence in
docs/logs/2026-10-mygo-observation/<iteration>/.
**Consumes:** MY-2 installed candidate and W7 owner-accepted installed baseline.
**Produces:** Native behavior/cost comparison with traceable environment and inputs.

- [ ] Pin each artifact's DIVA/VIVY/recipe/frontend/dependency/native-runtime
identity. Run each in separate clean checkouts/profiles with the same machine,
VRM and named workload; use sequential runs to avoid mutual interference.
- [ ] Record available cold/warm startup, idle/active memory, event recovery,
quit results and clean build/package observations with method and variance.
State the research Go 1.27.1 vs delivery 1.26.4 difference; an optional
same-compiler Wails run is a separately built/inspected control, not a silently
changed delivery baseline. Unavailable measurements remain pending.
- [ ] Count actual hand-maintained adapter changes separately from generated
bindings; record dependencies, generation/build/installer steps, drift failures,
upstream patches and actual upkeep effort. Do not predict a speedup from README
numbers or create an arbitrary percentage gate.
- [ ] Review correctness/native parity before cost benefits; produce a comparison
whose values link to raw evidence. W7 absence keeps this task blocked.

## Task 2: Record the decision and future upkeep rule

**Files:** observations.md, index.md, root TODOLIST.md and iteration logs.
**Consumes:** Task 1 evidence and MY-D1 promotion/trigger policy.
**Produces:** Observe, pause/drop, or proposed separate migration with reasons.

- [ ] Choose a recommendation from measured outcomes and missing evidence.
Observe is the default when evidence is incomplete; pause/drop if required
contracts/sealing are infeasible or upkeep exceeds the demonstrated benefit.
- [ ] For a switch recommendation present installed parity, maintained surface,
toolchain/native costs, migration delta and rollback implications to the owner.
Record their actual decision; absent approval leaves the branch research-only.
- [ ] For future relevant releases/mainline interface changes append a trigger
row, pin new inputs, map invalidated tests and reconcile only affected adapter
paths. Execute only the affected authorized gates; preserve previous evidence.
- [ ] Verify links/status/evidence consistency, commit the focused report and
release LOCK.md. No automatic merge, scheduled monitor or framework promotion.

## Acceptance and handoff

A durable, evidence-linked recommendation is complete even when the outcome is
continued observation. Framework migration, new public abstractions, distribution
changes and scheduled monitoring need their own requested scope. No promise of
continuous background work is made.
