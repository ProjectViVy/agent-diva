# Documentation policy alignment

Date: 2026-10-11. DIVA origin/main:
`0a64d1ce152440f1f63d4224a155c4616d2aff5b`.
Branch: `docs/memory-policy-alignment-20261011`.
Worktree: `/workspace/doc-alignment/agent-diva`.

## Outcome

Added the current [memory-policy authority](../../../architecture/memory-policy-alignment.md)
and reconciled selected-policy requirements, S05/S08/S09 and V10/V18/V19/V21.
Current architecture specifies one free-form subject Recap, revision conflicts,
commit-before-raw-context-unload, full editable UI, targeted active deletion,
deferred explicit Agent tools, human frozen workflow, role-only worker limits,
and BML-first/owner Mentle validation with one capability-aware abstraction.
VIVY controls admission/scope/budgets through its existing runtime/context path.
No implementation, migration, backend switch, dual write or source repin occurred.

Closed former PR/dependency and CI follow-ups using merged PR #5/#46/#48 and
exact VIVY `ce33bc8776b9f52ab13bed5f2cdaf8ebad01f666` run 38095646661 (all four
checks successful, including the supported DIVA memory lane). Documented the
separate unresolved DIVA consumer compatibility with VIVY's retired host/pack
seams. Corrected only the documentation package's stale S03 source inventory.

## Changed/current inventory

- New current authority: `docs/architecture/memory-policy-alignment.md`.
- Current policy and acceptance: memory-loop requirements, README, S05/S08/S09,
  verification matrix, current source inventory in S03/package-map.json,
  p0 architecture, backend contracts and runbook compatibility limitation.
- Current root backlog: TODOLIST records Recap/UI/tools, backend/product
  validation, consumer compatibility and preserved author-attribution follow-up.
- Current entry links/notices: root guidance/READMEs/LAPUTA, DIVA Next index,
  current memory-loop package and Wails plans point to the same authority.
- Earlier DN and closure planning baselines receive additive selected-policy
  notices; archive/defer receives a historical-status notice only.
- Required iteration logs and scoped LOCK handoff record this delivery.

Exact edited tracked paths before this log was created:

- `AGENTS-ARCH.MD`
- `AGENTS.md`
- `LAPUTA.md`
- `LOCK.md`
- `README.md`
- `README.zh-CN.md`
- `TODOLIST.md`
- `defer.md`
- `docs/plans/diva-next/DN-0.md`
- `docs/plans/diva-next/DN-1.md`
- `docs/plans/diva-next/DN-2.md`
- `docs/plans/diva-next/DN-3.md`
- `docs/plans/diva-next/DN-4.md`
- `docs/plans/diva-next/DN-5.md`
- `docs/plans/diva-next/DN-6.md`
- `docs/plans/diva-next/DN-7.md`
- `docs/plans/diva-next/DN-8.md`
- `docs/plans/diva-next/DN-L.md`
- `docs/plans/diva-next/DN-M.md`
- `docs/plans/diva-next/DN-P.md`
- `docs/plans/diva-next/DN-W.md`
- `docs/plans/diva-next/backend-separation-contracts.md`
- `docs/plans/diva-next/closure/DN-0C.md`
- `docs/plans/diva-next/closure/DN-0P.md`
- `docs/plans/diva-next/closure/DN-0S.md`
- `docs/plans/diva-next/closure/DN-2A.md`
- `docs/plans/diva-next/closure/DN-2B.md`
- `docs/plans/diva-next/closure/DN-4A.md`
- `docs/plans/diva-next/closure/DN-4B.md`
- `docs/plans/diva-next/closure/DN-4C.md`
- `docs/plans/diva-next/closure/DN-4D.md`
- `docs/plans/diva-next/closure/DN-6A.md`
- `docs/plans/diva-next/closure/DN-6B.md`
- `docs/plans/diva-next/closure/DN-6C.md`
- `docs/plans/diva-next/closure/DN-8C.md`
- `docs/plans/diva-next/closure/DN-LC.md`
- `docs/plans/diva-next/closure/DN-M-C.md`
- `docs/plans/diva-next/closure/DN-P-C.md`
- `docs/plans/diva-next/index.md`
- `docs/plans/diva-next/memory-loop-verification.md`
- `docs/plans/diva-next/memory-loop/README.md`
- `docs/plans/diva-next/memory-loop/continuity.md`
- `docs/plans/diva-next/memory-loop/contracts.md`
- `docs/plans/diva-next/memory-loop/epics/E01.md`
- `docs/plans/diva-next/memory-loop/epics/E02.md`
- `docs/plans/diva-next/memory-loop/epics/E03.md`
- `docs/plans/diva-next/memory-loop/epics/E04.md`
- `docs/plans/diva-next/memory-loop/epics/E05.md`
- `docs/plans/diva-next/memory-loop/package-map.json`
- `docs/plans/diva-next/memory-loop/requirements.md`
- `docs/plans/diva-next/memory-loop/runbook.md`
- `docs/plans/diva-next/memory-loop/stories/S01.md`
- `docs/plans/diva-next/memory-loop/stories/S02.md`
- `docs/plans/diva-next/memory-loop/stories/S03.md`
- `docs/plans/diva-next/memory-loop/stories/S04.md`
- `docs/plans/diva-next/memory-loop/stories/S05.md`
- `docs/plans/diva-next/memory-loop/stories/S06.md`
- `docs/plans/diva-next/memory-loop/stories/S07.md`
- `docs/plans/diva-next/memory-loop/stories/S08.md`
- `docs/plans/diva-next/memory-loop/stories/S09.md`
- `docs/plans/diva-next/memory-loop/stories/S10.md`
- `docs/plans/diva-next/memory-loop/stories/S11.md`
- `docs/plans/diva-next/memory-loop/stories/S12.md`
- `docs/plans/diva-next/memory-loop/stories/S13.md`
- `docs/plans/diva-next/memory-loop/verification.md`
- `docs/plans/diva-next/p0-design.md`
- `docs/plans/diva-next/wails/W0.md`
- `docs/plans/diva-next/wails/W1.md`
- `docs/plans/diva-next/wails/W2.md`
- `docs/plans/diva-next/wails/W3.md`
- `docs/plans/diva-next/wails/W4.md`
- `docs/plans/diva-next/wails/W5.md`
- `docs/plans/diva-next/wails/W6.md`
- `docs/plans/diva-next/wails/W7.md`

## Preserved historical inventory

All pre-existing `docs/logs/**`, archived backlog checkboxes, source/artifact
pins, fixtures and raw/hash evidence are unchanged. Baseline SHA tables remain
as originally recorded. Additive notices do not upgrade old proof to new policy.

Selected preserved evidence:

- [v3.0 recall/correction/deletion](../../2026-10-memory-loop-chain/v3.0.0-s08-s11-closeout/summary.md).
- [v3.1 historical ACTMEM/WORLD boundary and Agent-tool gap](../../2026-10-memory-loop-chain/v3.1.0-s08-v19-boundary/summary.md).
- [C01 terminal-before-capture](../../2026-10-memory-loop-chain/v2.3.0-s10-c01-terminal-before-capture/summary.md).
- [C02 capture-before-ACK](../../2026-10-memory-loop-chain/v2.2.0-s10-c02-capture-before-ack/summary.md).
- [C03 canonical-before-receipt](../../2026-10-memory-loop-chain/v2.4.0-s10-c03-canonical-before-caller-receipt/summary.md).
- [C04 partial effect batch](../../2026-10-memory-loop-chain/v2.5.0-s10-c04-partial-effect-batch/summary.md).
- [C05 effects-before-watermark](../../2026-10-memory-loop-chain/v2.6.0-s10-c05-effects-before-watermark/summary.md).
- [C06 canonical-before-index](../../2026-10-memory-loop-chain/v2.7.0-s10-c06-canonical-before-index/summary.md).

## Unresolved work

Authoritative subject identity; Recap CAS/conflict/recovery and human UX;
post-active-delete consolidation behavior; capability-aware backend validation;
deferred discovery mechanics and tool authority; measured budgets;
Work/reconcile/capture compatibility; and DIVA replacement consumer seam.
New policy/UI implementation and installed Windows/real-model,
unknown/partial-effect, backend recovery and workspace matrices remain open.
No Story state, historical acceptance result or candidate pin is promoted.

## Final link-review follow-up

Converted four pre-existing dead `LAPUTA.md` links into explicit historical
path references. The `docs/architecture/laputa/` directory is absent; references
are preserved as text, and the current memory-policy link remains usable.
No historical evidence or original architectural decision is rewritten.
