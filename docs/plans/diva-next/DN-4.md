# DN-4 — Companion-domain frontend handoff

- **Epic:** B · **Requirements:** R-4 · **Outcome:** persona edit reaches captured/live model input; memory persists/retrieves across restart; AutoDream can cancel/recover; accepted changes affect later runs; reports are real artifacts.
- **Authoritative design:** issue #13 DN-P1 §3–§4 · **Baseline:** `0fd005a1` · **Status:** see index; per-domain verified contracts required · **Predecessor:** DN-2 · **Index:** [index.md](index.md)
- **Files:** `src/components/PersonaMemoryView.vue`, `PersonaSetupGate.vue`, `components/memory/MemoryView.vue`, `EvolutionView.vue`, `NotebookView.vue`, `components/persona-memory/`, related service/state code. **Escalate:** any pressure to ship a fake-success adapter — forbidden by the issue.

## Prerequisites / contracts

- Consumes per-domain #63 contracts as they land; DN-0 mappings for each companion surface.
- VIVY prompt-only mask semantics preserved; old model/tool controls mapped separately.

## Tasks (per available domain only)

- [ ] Integrate persona and its actual prompt projection; expose revision conflicts/history and exact accepted edits.
- [ ] Integrate memory + ACTMEM/MEMRULES/continuity per accepted backend contracts.
- [ ] Bind AutoDream status/events/cancel/recovery, skill/persona review, notebook/report results.
- [ ] Remove old Rust service consumers for each domain after real verification.

## Verification

- `pnpm --dir agent-diva-gui test` and `build` pass.
- Per-domain evidence: persona edit visible in model input; memory survives restart; reflection cancel/recover works; reports resolve to real artifacts.

## Notes

Work may proceed per available domain; domains without a #63 contract stay Blocked with the missing API named — never mocked.

## P0-D1 execution amendment (takes precedence)

### Concrete P0 domain slices

All frontend paths above are relative to `agent-diva-gui/`. Consume P0-D1 and DN-0's pinned schema/fixture for each of persona, memory/ACTMEM/MEMRULES, evolution/AutoDream/review, notebook/report and masks. Existing VIVY plugins are evidence to inspect, not proof of frontend semantic parity. The prior #63 reference is historical; inaccessible issue metadata is not evidence that code is missing.

- [ ] For each domain, pin its actual frontend symbols in the ledger and add tests next to those components for read/edit/validation/conflict and missing capability. Test accepted edits against subsequent backend model input or persisted artifact as applicable.
- [ ] Replace old desktop.ts/domain consumers with DN-1 calls; preserve distinct persona, memory and governed evolution authority. Do not route one domain's approval into another domain merely because names resemble each other.
- [ ] Implement event/snapshot reconciliation with backend IDs; test restart and pending review recovery. AutoDream cancellation must cancel actual backend work; stale revisions must be rejected visibly.
- [ ] Record model-input/readback/artifact evidence per required row and run GUI test/build. Remove mock-backed production branches only with accepted replacements.

Readiness blocker: exact verified per-domain schemas, action IDs and acceptance fixtures from DN-0. If absent, record the producer contract needed and keep that slice Blocked; no speculative backend implementation is delegated by this plan.
