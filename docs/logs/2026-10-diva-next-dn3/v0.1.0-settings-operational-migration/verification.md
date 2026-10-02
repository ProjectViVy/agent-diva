# Verification

## Automated
- `npx vitest run` — 434/434 pass (54 files); count dropped vs pre-DN-3 as
  retired-surface tests were deleted with their views.
- `npx vue-tsc --noEmit` — clean.
- `pnpm build` — clean.

## Live persist/reopen acceptance (sealed vivy-shared.so, ctypes)
`~/diva-dn3-accept/persist.py` — 10/10 PASS:
- `settings/update` (sandbox.allowed_domains marker + approval_timeout +
  http.timeout_seconds) → same-session `settings/get` shows writes.
- VivyShutdown → fresh VivyInit+initialize → `settings/get` returns all
  persisted values (true cross-process persistence).
- Runtime facts (workspace_root) present; bogus preset rejected `-32602`
  fail-closed with state unchanged.

## Not covered
- GUI-level window reopen against live settings (needs webkit runner).
- Windows/amd64 native acceptance (P0-NATIVE-VERIFICATION still open).
