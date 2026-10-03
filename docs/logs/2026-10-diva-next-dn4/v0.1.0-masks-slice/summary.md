# DN-4 — Companion domain dispositions + masks slice

## Per-domain contract verification (compiled `diva` generation, 20 modules)

Evidence base: `agent-vivy@feat/diva-embedded` — `internal/rpc/control.go` method table, `internal/modules/*/actions.go`, `plugins/*/module.go`, staged `generation.json` / `zz_assembly.go`.

| Domain | Verdict | Producer contract |
|---|---|---|
| masks | **Implemented** | `vivy/masks` module compiled; actions `vivy.masks.catalog.{list,get,create,update,delete}`, `vivy.masks.selection.{get,set}` via `module.action.invoke`. Error codes in `error.data.code` incl. `revision_conflict`, `mask_in_use`, `mask_unavailable`, `incompatible_prompt_version`. |
| persona | Blocked | `plugins/vivy-persona` provides only `UIExtensionPort` (`vivy.persona.sidebar` etc.) — zero backend actions. Needs a compiled module with persona read/edit actions + revision semantics + prompt projection. |
| memory / ACTMEM / MEMRULES | Blocked | `internal/modules/memory` exists with `vivy.memory.{list,search,get,add,update,remove,rules.read,rules.write,status}` but is **not compiled** into the diva recipe. Needs recipe inclusion; ACTMEM/continuity producer does not exist at all. |
| autodream / evolution / review | Blocked | `plugins/vivy-evolution` is UI-extension only; no autodream producer module exists. Needs status/event/cancel/recovery actions bound to real backend work. |
| notebook / reports | Blocked | `plugins/vivy-notebook` is UI-extension only; no reports producer module exists. Needs notebook CRUD + report artifact actions producing real artifacts. |

## Masks slice delivered

- `src/api/masks.ts` — typed adapter over `module.action.invoke`; `MaskActionError` unwraps `error.data.{code,current_revision,reference_count}`; create uses crypto `operation_id` (UUID); `listAllMasks` paginates with a defensive empty-page stop.
- `src/components/settings/MasksSettings.vue` — catalog list (built-in/active badges), selection set per session via `vivy.masks.selection.set` with `expected_revision`, create/edit/delete with expected-revision conflict resync (fresh revision pinned at edit start; conflict discards draft and reloads), unavailable state when the module reports `not_compiled`.
- Wired into `SettingsView`, `SettingsDashboard` (masks card), `NormalMode` union, en/zh locales.
- Tests: `src/api/masks.test.ts` (9) + `MasksSettings.test.ts` (7) — routing, pagination, revision discipline, conflict paths, missing-capability UI. `pnpm test` 450/450, `vue-tsc` clean.

## Live contract outcome

- masks contract driven live on the sealed .so (list/get/selection.get+set/catalog.create) — see verification.md.
- Two embedded-path backend gaps found and handled: `2947c472` binds a caller identity on the `DialControl` serving peer (module actions were unreachable); effectful `vivy.masks.*` writes additionally need governance allow-rules (no embedded approval continuation) — deployment dependency recorded in TODOLIST `MODULE-ACTION-GOVERNANCE`.
- The shipped runtime was re-sealed from `agent-vivy@2947c472` (generationId `1fd14fb2…`) and DN-P evidence re-run against the new bytes.

## Deferred

Persona/memory/ACTMEM/autodream/notebook/reports stay Blocked until their producer contracts are compiled into the diva generation — named above; no mocks were shipped.
