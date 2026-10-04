> 2026-10-03 scope amendment: [DN-C2](p0-design.md) and [index.md](index.md)
> take precedence over historical P0-D1 host/speech/import/acceptance premises.
> Tasks/evidence below retain their original scope and artifact pins. New
> closure work is not proved by historical implementation or acceptance.

# DN-3 — Settings and operational surfaces

- **Epic:** B · **Requirements:** R-1, R-3 · **Outcome:** required settings/operational forms control the actual VIVY process after restart.
- **Authoritative design:** issue #13 DN-P1 §3–§4 · **Baseline:** `0fd005a1` · **Status:** see index · **Predecessor:** DN-2 · **Index:** [index.md](index.md)
- **Files:** `agent-diva-gui/src/api/{desktop,providers,tokenStats,capabilities}.ts`, `src/components/SettingsView.vue`, `src/components/settings/`, `CronTaskManagementView.vue`, planning surfaces, related `App.vue` config handlers. **Escalate:** a required domain has no VIVY settings API (then it is a DN-0-recorded parity blocker).

## Prerequisites / contracts

- Consumes DN-0 dispositions for every settings domain; DN-2's verified VIVY mutation path.
- VIVY owns provider configuration and credentials: no legacy whole-config write, no frontend-held persistent credential authority, no direct speech-provider bypass.

## Tasks

- [ ] Migrate provider/model, tools, MCP, channels, skills, Cron and diagnostic/control pages in small domain slices against DN-0 mappings — one slice per reviewable change.
- [ ] Replace whole legacy config load/save with VIVY-owned operations + server validation; show configured vs available vs running states.
- [ ] Keep native appearance preferences separate from backend configuration.
- [ ] Remove each obsolete desktop DTO/command consumer as its replacement is verified; no general old-command-name compatibility facade.
- [ ] Verify persist/reopen, invalid configuration, unavailable providers, failed connections, permission handling.

## Verification

- `pnpm --dir agent-diva-gui test` and `build` pass.
- Restart the app: forms still drive the actual VIVY process; unsupported capabilities show as visibly unavailable (parity blocker if required by DN-0).

## Evidence to supervisor

Per-domain slice list with verification results; parity-blocker list if any.

## P0-D1 execution amendment (takes precedence)

### Concrete P0 domain slices

Consume P0-D1 ABI/transport and DN-0 per-domain exact schemas. Proposed tests `agent-diva-gui/src/api/vivy/operational.test.ts`; existing tests in `src/components/settings/` remain behavioral consumers. Work in this order: provider/model configuration and credential redaction; tool/MCP/skills; channels/Cron; diagnostics/token statistics/sandbox/command rules/marketplace. Include files/resources/masks wherever DN-0 locates their actual owner. Each slice is blocked independently if no matching backend capability is verified.

- [ ] Add a failing fixture-backed test for the slice's real read/mutation/error semantics; exercise invalid input and unavailable capability.
- [ ] Replace the named old consumer and data transformation from the ledger with the exact RPC/action, using DN-1 client only; remove its old event/DTO branch.
- [ ] Re-read the backend after mutation and restart; prove the value changes actual runtime behavior, not just form state. Test revision conflict and secret redaction where applicable.
- [ ] Run GUI test/build commands above and record ledger row IDs, source diff and backend readback. Commit one domain slice; stop on unknown action semantics rather than inventing an alias.

Exact target action IDs are deliberately a DN-0 prerequisite, not guessed here. This plan cannot become Ready for a domain until its file/symbol/schema rows and fixtures are present. No additional service or generic legacy-command dispatcher is authorized.
