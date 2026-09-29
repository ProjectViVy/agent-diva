# DN-4 — Companion-domain frontend handoff

- **Epic:** B · **Requirements:** R-4 · **Outcome:** persona edit reaches captured/live model input; memory persists/retrieves across restart; AutoDream can cancel/recover; accepted changes affect later runs; reports are real artifacts.
- **Authoritative design:** issue #13 DN-P1 §3–§4 · **Baseline:** `0fd005a1` · **Status:** Blocked — missing persona projection, working-memory, AutoDream/review or report APIs belong to #63 (each domain individually gated) · **Predecessor:** DN-2 · **Index:** [index.md](index.md)
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
