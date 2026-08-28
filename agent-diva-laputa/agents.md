# agent-diva-laputa

## OVERVIEW

File-first authority storage for the BML (Basic Memory Layer). Owns the typed memory store, persona workspace, Frozen Core snapshot, actmem/feedback stores, and the proposal/apply boundary.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| Typed memory authority store | `src/bml.rs` (`TypedMemoryStore`, `MemoryHome`, `MemoryAdapterContext`) |
| Actmem (activity memory) | `src/actmem.rs`, `src/actmem_edit_work.rs`, `src/actmem_item.rs` |
| Persona workspace / proposals | `src/persona.rs` (`PersonaService`, `PersonaDocument`, `PersonaChangeRequest`) |
| Frozen Core snapshot | `src/frozen_core.rs` |
| Recall / feedback reader | `src/feedback.rs` |
| Memory record types | `src/memory_records.rs` (`LaputaSection`, `SectionStatus`) |
| Layout / paths / storage | `src/layout.rs` (`LaputaPaths`, `LaputaStorage`) |
| Atomic write helpers | `src/atomic.rs` |
| Locking | `src/lock.rs` |
| Error type | `src/error.rs` (`LaputaError`) |

## CONVENTIONS

- All durable BML writes go through `TypedMemoryStore` / `PersonaService` and are governed by proposals.
- Read-only recall and feedback are served by the `feedback` module.
- Layout paths are centralized in `src/layout.rs`; do not scatter path construction.

## ANTI-PATTERNS

- **Never** call BML write APIs (`put`/`put_governed`/`import_records`/`rollback_governed`) directly from governance modules; use `apply_proposal`.
- Do not let `agent-diva-autodream` or `agent-diva-agent` write identity/MEMORY.md directly.
- Do not treat legacy Markdown files as the authority; they are offline import sources.
- Do not bypass `LaputaLock` when mutating shared stores.

## NOTES

- `just bml-boundary-check` enforces that governance modules do not call BML write APIs directly.
- `just laputa-clean-break-check` verifies that legacy `mentle`/`memtle` names have not re-entered active code.
