# DN-3 — Settings and operational surfaces

- **Epic:** B · **Requirements:** R-1, R-3 · **Outcome:** required settings/operational forms control the actual VIVY process after restart.
- **Authoritative design:** issue #13 DN-P1 §3–§4 · **Baseline:** `0fd005a1` · **Status:** Planned · **Predecessor:** DN-2 · **Index:** [index.md](index.md)
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
