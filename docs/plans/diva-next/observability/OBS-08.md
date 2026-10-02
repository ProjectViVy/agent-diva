# OBS-08 — DIVA diagnostics and visible GUI persistence Implementation Plan

> **For agentic workers:** After specification review and implementation authorization, use `superpowers:executing-plans`. Read this plan and OBS-D1 first. Delegation follows repository instructions, not this heading alone.

**Goal:** Replace removed log commands and swallowed GUI writes with bounded diagnostics and explicit persistence status.
**Architecture:** Typed wrappers use OBS-05 producer; GUI recorder state is separate from captured console output.
**Tech stack:** Vue/TypeScript, Vitest, existing guiLogger and accepted shared client.
**Epic / requirements:** OBS-C / O5,O7
**Specification:** [architecture.md](architecture.md), OBS-D1. **State/evidence:** [parent index](../index.md).
**Immediate inputs:** OBS-05: accepted diagnostic RPC/limits/error fixtures; DN-2: real shared client and window/app lifetime.
**Global constraints:** one VIVY authority; existing Eino execution and Recipe/Generation path; no legacy fallback, production data, synthetic usage, guessed wire adapters or fabricated acceptance. VIVY code changes require its AGENTS and pinned-capability check. DIVA writers claim LOCK.md before edits and serialize shared files.
**Path convention:** VIVY paths are relative to the selected agent-vivy checkout; DIVA paths to agent-diva. New paths explicitly marked proposed. Source baselines are in OBS-D1.

## Files and exact seam

Modify DIVA `agent-diva-gui/src/components/settings/audit/{AuditPage.vue,AuditPage.test.ts}`, `src/utils/guiLogger.ts` and proposed `guiLogger.test.ts`, `src/api/capabilities.ts`; extend shared observability wrapper/contracts sequentially. Page/route labels become Diagnostics. Preserve explicit unresolved DN-0 audit mapping rather than an old DTO alias.

Wrappers `readDiagnostics(client: VivyClient,q: DiagnosticQuery): Promise<DiagnosticPage>` and `appendGuiLogs(client: VivyClient,batch: GuiLogBatch): Promise<GuiLogAck>` map exactly to D5. Local recorder state `{queued:number,dropped:number,persisted:number,last_error?:string}` is presentation evidence, not durable audit state. Queue/batch bounds match accepted producer bounds.

## Ordered tasks

- [ ] Add tests for runtime/GUI page mapping, source/date/filter/cursor semantics, empty/malformed/clipped/gap/error states and absent capability. Remove get_audit_events/get_gateway_log_lines/get_gui_log_lines after their ledger dispositions are recorded.
- [ ] Implement typed bounded page requests, manual refresh/load-next and visible rotation gap/reload. Never pass a filesystem path from the browser. Per-run activity links to trajectory; global audit is explicitly unavailable/pending, not rebranded log lines.
- [ ] Add guiLogger tests for sanitization, bounded queue, dropped count, write acknowledgment, failure, partial accepted count, disconnect and detach/reopen. Remove append_gui_log and silent catch-success semantics.
- [ ] Send batches only via the accepted client; acknowledge persisted count only on producer ack. Surface errors in recorder state/UI without console logging recursion. Do not blindly retry partial writes or add durable frontend storage as a second log authority.
- [ ] Keep diagnostics failure isolated from chat. Verify reader view cannot feed its own response/errors into an endless recorder loop. Window close detaches UI only; explicit app exit follows the DN-L/DN-5 owned close policy.
- [ ] Commit `feat(diagnostics): connect diva logs and gui persistence state`.

## Verification and handoff

`pnpm --dir agent-diva-gui exec vitest run src/components/settings/audit/AuditPage.test.ts src/utils/guiLogger.test.ts`, full GUI `test` and `build`. Expected red: existing swallowed failures/deleted methods cannot satisfy acknowledgment tests. Expected green: gaps, redaction, bounded queues and error isolation assertions pass.

Native smoke with temp logs: write GUI sentinel secret + safe event, observe sanitized persisted event via diagnostics/logs, induce write/unavailable failure, rotate the family and reopen the window. No production log access. Return producer/client fixture version, limits, core identity, source commit, commands and unresolved global-audit ledger row to OBS-09. Review focus: no recursion, truthful partial writes, unknown global audit and secret-free diagnostics.
