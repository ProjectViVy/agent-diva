# DN-7 — Offline data handoff and rollback safety

- **Epic:** D · **Requirements:** R-7 · **Outcome:** repeat import without duplication; partial-failure recovery; references/assets resolvable; source data untouched; rollback uses the preserved old home/release.
- **Authoritative design:** issue #13 DN-P1 §4 (Data rules) · **Baseline:** `0fd005a1` · **Status:** Blocked — versioned import contracts and voice/resource asset schema where imported · **Predecessors:** DN-3, DN-4 · **Index:** [index.md](index.md)
- **Inspect:** `agent-diva-migration/`, `agent-diva-core/src/session/`, `agent-diva-laputa/src/bml/memory_home.rs`, `agent-diva-laputa/src/persona/`, `agent-diva-core/src/evolution/skill_home.rs`, asset/config formats. **Proposed:** `tests/migration/` fixtures, `docs/plans/diva-next/data-mapping.md`; importer ownership/API coordinated with #63. **Escalate:** missing target schema for a domain → that domain's import is Blocked, not improvised.

## Prerequisites / contracts

- Consumes DN-3/DN-4 accepted domain contracts; offline snapshot of a stopped source (incl. database sidecars); no live Rust bridge, no background dual writes.
- Import produces versioned identity/count/error records; retry = no duplication; credentials re-entered/rebound through the new backend; source secrets never in reports.

## Tasks

- [ ] Map sessions/messages, persona/history, memory/ACTMEM/MEMRULES, skills, schedules, settings, required assets into `data-mapping.md`; disposition unsupported records explicitly.
- [ ] Implement/consume dry-run, versioned importer with source→target identity mapping, counts, errors, repeat-import receipts.
- [ ] Verify old data untouched; document that returning to the old release does not include new-system writes (exporting those is a separate compatibility operation).

## Verification

- Representative populated fixtures; repeat import; partial-failure recovery; read-only source checks.

## Evidence to supervisor

`data-mapping.md`, fixture list, import receipt samples, read-only source check output.
