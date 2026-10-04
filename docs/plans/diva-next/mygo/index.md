# MyGo observation branch plan — MY-P1

**Design:** [MY-D1](design.md). **Branch:** `research/mygo`.
**Epic:** MY-E1 — Keep a small, reproducible native-host alternative under observation.
**Authorization:** owner authorized inline implementation on 2026-10-04.
Standalone MY-0 probe work has started; product implementation is gated below.
Wails remains the delivery path. This package adds no prerequisite to W0–W7.

## Stories and readiness

This table is the single authority for Story status and dependency data.
External W Stories refer to the existing accepted-output gates, not plan publication.

| Story | Requirements | Deliverable | Immediate prerequisites and output | Plan | Status / blocker |
| --- | --- | --- | --- | --- | --- |
| MY-0 | MY-R1, MY-R4, MY-R6 | Pinned feasibility/observation baseline and exact native API/build notes | None | [MY-0](MY-0.md) | Partial: Linux native probe and standalone build/generation passed; Windows/WebView2 gate pending |
| MY-1 | MY-R2, MY-R3, MY-R4 | Sealed thin MyGo host with existing Agent transport | MY-0 GO; W1 accepted public Host; W2 sealed external pack; W3 accepted frontend/native seam | [MY-1](MY-1.md) | Blocked: prerequisites unaccepted at inspected baseline |
| MY-2 | MY-R3, MY-R4 | Native speech/VRM and installed Windows parity evidence | MY-1 candidate; W4 accepted Go speech interfaces; W6 accepted packaging-only pipeline | [MY-2](MY-2.md) | Blocked: candidate, speech and packaging evidence missing |
| MY-3 | MY-R1, MY-R5, MY-R6 | Cost comparison and durable observe/pause/promotion recommendation | MY-2 installed candidate; W7 accepted installed Wails baseline | [MY-3](MY-3.md) | Blocked: comparable installed-product evidence missing |

Local waves: {MY-0}, {MY-1}, {MY-2}, {MY-3}. External acceptance can delay a wave.
The chain has three edges; no duplicated transitive edges. W1/W2/W3 inputs are
separate interface, compiler and frontend/native outputs, so all are genuine
external prerequisites. W4/W6/W7 likewise supply separate accepted outputs.

A GO in MY-0 is feasibility for this bounded experiment, not a framework switch.
Linux native feasibility evidence is recorded in [run 001](run-001.md).
Installed-product, Windows/WebView2 and configured-provider evidence is pending.
Readiness requires accepted predecessors and current contract/source
reconciliation. An inaccessible native environment blocks affected gates only.

## Execution and file ownership

Execute sequentially using superpowers:executing-plans under the owner authorization.
No delegation is selected by this plan. MY-0 edits observation records/probes;
MY-1 owns host, IPC and research build integration; MY-2 owns native speech/media
adaptation; MY-3 owns comparison/decision records. Shared go.mod/go.sum,
desktop-host.ts, scripts/build-desktop.py and observation/index edits are serialized.
Use root LOCK.md and a dedicated branch checkout; preserve unrelated mainline work.

Every implementation task ends with scoped verification/evidence and an English
human-attributed Conventional Commit. Follow current repository iteration logs;
do not copy changes from another branch's lock state into its live workspace.

## Coverage and handoff

The table covers all six design requirements. Ten reviewable tasks are defined:
two in MY-0, three in MY-1, three in MY-2, two in MY-3.
Every Story includes file scope, interfaces, failure behavior and verification.
Proposed source paths are clearly distinguished from verified baseline paths.

Hand off the Story plan, MY-D1, the pinned external W3 contract, accepted
predecessor evidence, the [observation ledger](observations.md), source lock and
current repository rules. If a public contract or material scope changes, stop
the affected Story and revise its downstream plans before implementation.

## Branch upkeep and promotion

Preserve this docs directory when integrating an accepted mainline baseline.
Reconcile prototype changes at public interface/framework trigger points.
Use a small adapter patch and reproducible build inputs instead of carrying
a second production product. No routine merge-per-commit requirement exists.
Every new upstream candidate gets fresh pins and an explicit affected-gate map.

An owner-approved promotion is a new migration decision/plan with a reviewable
delta from Wails. Choosing pause/drop simply parks prototype work and preserves
evidence. Source comparison alone does not qualify a candidate for promotion.

## Publication evidence

Planning-only static checks and publication details are recorded in
[verification](../../../logs/2026-10-mygo-observation/v0.1.0-plan/verification.md)
and [acceptance](../../../logs/2026-10-mygo-observation/v0.1.0-plan/acceptance.md).

## Implementation evidence

[Run 001](run-001.md) and [v0.2.0 verification](../../../logs/2026-10-mygo-observation/v0.2.0-probe/verification.md) record the reproducible framework probe.
MY-0 is not GO until Windows evidence exists. MY-1–MY-3 are blocked by the
accepted-output dependencies above; no dummy SDK or product host was added.
