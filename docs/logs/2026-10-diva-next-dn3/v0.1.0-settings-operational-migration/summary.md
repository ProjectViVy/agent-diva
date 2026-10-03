# DN-3 — Settings / operational migration (slices A–D)

Scope: docs/plans/diva-next/DN-3.md. Every settings/operational surface now
talks to VIVY through `vivy_call` only — no legacy desktop DTO/invoke seam
remains outside `api/vivy`.

## Slice A — provider/model + credential redaction (ee01962b)
- `settings.ts` adapter over `settings/providers|get|update|model/select|refresh`;
  ProviderSettingsSnapshot + derived ConfigStatusReport (`buildStatusReport`).
- ProvidersSettings/ModelSelector migrated; API key is write-only (mask on
  read), error envelope kinds surfaced verbatim.

## Slice B — tools / MCP / skills / marketplace (204a9c23)
- MCP: `settings/mcp|upsert(FULL-REPLACE)|delete|probe`; env_from = child→host
  name mapping, never values.
- Tools: `tools/list` + `tools/set-active`; network roster is backend-owned.
- Skills: `skills/list|set-enabled` (CAS base_hash, -32009 stale → re-list);
  marketplace search|featured|install|check. Upload/delete/edit/history/
  skill-request have no VIVY RPC → blocked (TODOLIST SKILL-MANAGEMENT-GAP).
- Compaction + network search via `settings/update` sub-objects.

## Slice C — channels / cron (ad33f349)
- Channels: `channel/inspect` (process truth) + `channel/get|update` (document
  truth, applies next restart). Editable envelope = {enabled, allow_from,
  token_env} only; empty allow_from is a valid fail-closed write. Whole
  wizard/editor/card family (~3.4k LOC) deleted — per-platform plugin
  Settings is opaque.
- Cron: `cron/list|create|update(full-replace)|delete|trigger|stop`.

## Slice D — diagnostics / sandbox / residual purge (011e493c)
- `stats/tokens` → ConsoleView is stats-only (gateway/log/config panels gone).
- Sandbox: `settings/get.sandbox` + `settings/update.sandbox` overlay;
  workspace_root/execute_allowed_commands are read-only runtime facts;
  presets cautious|smart|trusted.
- Command-rules surface deleted — no VIVY approval-rule RPC
  (`commands/list` is slash commands) — COMMAND-RULES-GAP.
- Mass dead-surface purge (-12.5k LOC): autodream/evolution/skill-request
  governance cards, personas, BML memory, notebook, masks, audit/gui logs,
  self-evolution settings, subagent panel, PersonaSetupGate,
  set_splash_complete, upload_file + attachment UI, storybook invoke mocks.
- GeneralSettings: status card from `loadProviderState`; danger zone is
  localStorage-only wipe (BACKEND-WIPE-GAP).
- `api/desktop.ts` is now pure view-model types + `isTauriRuntime`.
  `rg "invoke("` outside api/vivy = only the dormant pet feature (DN-6).
