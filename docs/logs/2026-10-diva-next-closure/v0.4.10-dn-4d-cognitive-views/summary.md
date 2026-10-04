# v0.4.10 — DN-4D cognitive setup + scoped companion views

Scope: `docs/plans/diva-next/closure/DN-4D.md`. Connects the captured
`diva.cognitive.*` surface (DN-4C fixture) to real companion views.
Also backfills the v0.4.9 iteration log for DN-6C (`f993afc4`), which
landed without one.

## What changed (agent-diva-gui)

- `api/cognitive.ts` — closed 20-action `CognitiveClient` over
  `module.action.invoke` on the existing VivyClient (never desktop
  native commands): `CognitiveOutcome<T>` envelope validation,
  safe-integer input check, read vs mutation classification, mutation
  timeout → `{status:'unknown', outcome_unknown, retryable:false}`,
  host-owned transport denials rethrown verbatim. DTO aliases only —
  no second domain dialect.
- `state/vivy-cognitive.ts` — session-scoped projection controller:
  epoch fencing discards stale responses after session change; drafts
  retained across save failures/timeouts; explicit authoritative
  readback (`reconcilePersona`); needsSetup / capabilityBlocked /
  frozenRecoveryRequired computed from the bound status.
- `persona-memory/PersonaSetupGate.vue` — five required-file init form;
  creates a setup session through `vivyChat.newSession()` when unbound.
- `persona-memory/PersonaMemoryView.vue` — per-kind CAS edit + save,
  pending-count, reviews list + accept/reject, frozen state+revisions
  (kept distinct from current persona revisions), scoped ACTMEM
  Pulse/Recap/Work + owner whole-document view.
- `components/MemoryView.vue` — card search, observed-revision expand,
  mutation with operation/status/canonical/index states, receipt
  lookup; scope/destination echoed from bound status only.
- `components/EvolutionView.vue` — cognition status block (phase,
  enabled, active_run_id, source, watermark, pending_through,
  block_reason, eligibility), policy CAS (enabled/min_interval_ms/
  base_revision), trigger, cancel, results; recovery_required shown
  and never cleared by policy writes.
- `App.vue` — binds the controller to `vivyChat.currentSessionId` on
  every controller sync; unbind on null.
- `ChatView.vue` — persona setup gate: send disabled + gate card above
  the input while `needsSetup`; settings/setup stay reachable.
- `SettingsView.vue` + `SettingsDashboard.vue` — persona/memory/
  evolution subviews + dashboard cards. Locales EN/ZH.

## Rulings (ledger: .superpowers/sdd/dn-4d-cognitive-views)

- Unknown write outcomes surface once and are never auto-replayed;
  `retryable:false` — recovery is an explicit read.
- CAS conflict retains draft + refetches authority; matching text alone
  cannot prove which operation committed.
- capability_unavailable keeps settings navigable with an honest block
  notice; it does not brick the chat send path — the gate is the setup
  requirement, surfaced only when a bound session reports
  uninitialized/incomplete persona.
