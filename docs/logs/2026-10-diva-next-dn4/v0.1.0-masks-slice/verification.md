# DN-4 masks slice verification

## Contract verification (read-only, compiled artifact)

- `module.action.invoke {module_id, action_id, input}` is the sole module control entry (internal/rpc/control.go:806-856); `MethodNotFound` when ActionHost unconfigured.
- `generation.json` (staged, shipped): 20 modules incl. `vivy/masks`; capabilityStates `channels:NOT_COMPILED, mcp:UNCONFIGURED`; generationId `322a9cbe…e9a4`.
- `internal/modules/masks/actions.go` — action IDs and input/output schemas quoted in summary.
- `plugins/vivy-{persona,notebook,evolution}/module.go` — each declares only `Provides: UIExtensionPort` → no backend actions exist for those domains.
- `internal/modules/memory/actions.go` — 9 `vivy.memory.*` actions defined but absent from the recipe's module list.

## Test evidence

- `npx vitest run src/api/masks.test.ts src/components/settings/MasksSettings.test.ts` — 16/16 PASS.
- `pnpm test` — 450/450 PASS (56 files).
- `npx vue-tsc --noEmit` — clean; `pnpm build` — clean.

## Behaviors covered by tests

- verbatim `module.action.invoke` routing with `module_id: 'vivy/masks'`;
- pagination via `next_after_id` + defensive stop on empty page;
- `operation_id` UUID generation on create; `expected_revision` enforced on update/delete/select;
- `MaskActionError` unwrap: `revision_conflict` carries `currentRevision`, `mask_in_use` carries `referenceCount`, untyped errors map to `code: null`;
- component: catalog load, no-session skip, active mark + select, revision-conflict resync (selection refetched), update conflict discards draft, built-in masks undeletable, `not_compiled` unavailable state.

## Live contract evidence (sealed .so, ctypes)

Driven against the shipped `vivy-shared.so` via `module.action.invoke`:

- `vivy.masks.catalog.list` → paginated items incl. `builtin/programmer`, `revision`, `digest`, `built_in`, `generation_id`.
- `vivy.masks.catalog.get` → full body for `builtin/programmer`.
- `vivy.masks.selection.get/set` → `builtin/programmer` bound to a fresh session, revision 0→1.
- `vivy.masks.catalog.create` → `custom/<uuid>` created (requires `description` — required field, verified by -32602 when omitted).
- Invalid input → `-32602`; unknown mask → error envelope (not silent).

## Backend fixes required by live evidence (landed in agent-vivy)

1. **`2947c472` embedded caller auth** — `DialControl`'s serving peer carried no Caller/Identity, so `module.action.invoke` always failed `action caller is not authenticated`. Fix binds the app session token + `face/embedded` identity on the host peer; ActionHost still validates `caller.Opaque == rpcToken`. Regression test `TestEmbeddedControlActionInvokesWithBoundCaller` added.
2. **Effectful actions need governance authorization** — `vivy.masks.*` writes (`selection.set`, `catalog.create/update/delete`) evaluate policy against `governance.profile` and have NO embedded approval continuation route, so they return `module action is not authorized` under every preset. Resolved by owner ruling ("align DIVA to vivy's style"): the shell writes `resources/vivy.default.yaml` on first launch when `<config_dir>/vivy.yaml` is absent; the template carries the four `vivy.masks.*` allow rules, which is the vivy-native mechanism (the embedded composition deliberately has no action-approval route). Live-verified: sealed .so boots on the shipped template and `selection.set` + `catalog.create` succeed; an existing user config is never overwritten (unit test `default_config_written_only_when_missing`).
