# Backend Separation Contracts — DN-0 Ledger (proposed)

Status: **PROPOSED — pending owner review**. This document freezes the DN-0
inventory and the ABI v1 contract record. Fixture capture is **pending** (no Go
runtime on the DN-0 host); `fixtures/core-rpc.json` asserts schema shape from
source and is not a wire transcript.

Baselines: agent-diva `d96e396d` (frontend source), retired backend at
`0fd005a1` (origin/dev), agent-vivy `5347032d8f` (checked out at
`/home/ubuntu/repos/agent-vivy`).

## 1. Inventory summary

Enumeration method: `rg -nU --multiline-dotall` for `invoke` (literal and
generic-parameter forms), `inv()`/`invokeCommand()` dynamic wrappers,
`listen(...)` registrations, `fetch(`/plugin HTTP calls, plus
`generate_handler!` extraction from `0fd005a1:agent-diva-gui/src-tauri/src/lib.rs`
(168 `commands::*` registrations). Every emitted command string was reconciled
against the registry; the registered-never-invoked set was reconciled by
subtraction. Regex counts alone are not completeness proof: the cross-check
below closes the loop.

| Metric | Count |
|---|---|
| Static + multiline `invoke`/`inv`/`invokeCommand` call sites | 158 |
| Unique command names invoked from `agent-diva-gui/src/` | 150 (144 literal + 6 wrapper-only) |
| Rust commands registered at `0fd005a1` but never invoked | 21 (dead registrations, below) |
| Invoked but never registered (phantom) | 1 (`list_subagent_results` — speculative mock IPC) |
| `listen(...)` event names across the frontend | 25 |
| Frontend listeners with no producer (either side) | 4 |
| Emitter with no listener | 1 (`emitTo('main','desktop-pet-voice-message')`) |
| Direct network calls bypassing invoke | 2 (STT fetch in `voice-api.ts:259`, TTS fetch in `tts-service.ts:697`) |
| Native Tauri APIs used without invoke | window drag/minimize, `plugin-opener` `openUrl`, local asset loads |

Registration-inventory anomalities:
- `set_splash_complete` is a plain `#[tauri::command]` fn registered directly in
  `lib.rs:193,317`, not under `commands::*` — counted as native.
- Dynamic invocation paths: `features/diva-pet/**` uses `inv()` (lazy binding)
  for `pet_read_vrm_model`, `pet_list_vrm_models`, `pet_import_vrm_model`,
  `pet_delete_vrm_model`; `services/tts-service.ts:879` uses
  `invokeCommand(name,args)` for `pet_siliconflow_synthesize` (3 sites) and
  `pet_minimax_synthesize` (1 site); `NotebookView.vue` does
  `await import('@tauri-apps/api/core')` inline.

Dead Rust registrations (21) — retire pending approval, see §6:
`approve_plan_report, delete_plan_todo, get_active_plan_execution,
get_channels, get_command_approvals, get_cron_job, get_execution_todos,
get_gateway_status, get_plan, get_runtime_info, get_service_status, greet,
install_service, resolve_command_approval, restore_plan_todo, start_service,
stop_service, tail_logs, uninstall_gateway, uninstall_service,
update_execution_todo`.

## 2. Core RPC surface — frozen from source

Verified against `internal/rpc/control.go` and `internal/rpc/protocol.go` at
`5347032d8f`. `ProtocolVersion = "vivy.rpc.v1"`. Wire frame: JSON-RPC 2.0 over
`JSONLTransport` (newline-delimited frames; `MaxFrameBytes` default 32 MiB,
`OutgoingBuffer` default 64). All handlers return a result object or
`*Error{code,message,data?}` with codes `-32700,-32600,-32601,-32602,-32603,
-32001` plus domain `CodeNotFound`/`CodeConflict` (`-32009` range family;
verify exact constant values during fixture capture).

| Method | Params (source-verified) | Result | Errors / notes |
|---|---|---|---|
| `initialize` | none | `{protocol_version:"vivy.rpc.v1", capabilities:[...], code_mode_available}` | capabilities array is the face's contract-discrimination source |
| `session/create` | `{title, workspace_path}` | `{id,title,created_at,updated_at,sandbox_mode,approval_policy,permission_preset,workspace_path}` | empty title → server-side auto-titler marks untitled |
| `session/list` | — | `{sessions:[sessionResult…]}` | |
| `session/get` | `{session_id}` | session metadata (not message history) | **not** a substitute for `session/messages` |
| `session/messages` | `{session_id, include_attachment_data?}` | `{messages:[messageResult…]}` | reconciles + applies view truncations before returning |
| `session/delete` | `{session_id}` | — | removal target; check residual-run behavior at capture |
| `session/rename` | `{session_id,title}` | — | |
| `session/rewind`, `session/fork`, `session/edit`, `session/context`, `session/compactions`, `session/sidebar`, `session/set_permission`, `session/set_workspace`, `session/todos`, `session/todo/update` | per-handler | — | planning/work surface, §5 |
| `turn/start` | `{session_id, text, mode?, attachments?, attachment_paths?, context_paths?}` | `{run_id, status:"accepted"}` | `RunAccepted` — accepted≠started; terminal arrives via events |
| `run/get` | `{run_id}` | run record `{…,status}` | statuses include `cancelled` |
| `run/cancel` | `{run_id}` | `{run_id,status:"cancelling"}` | `CodeNotFound` if run not active in this process — **restart window race**, see §7 |
| `run/log` | `{run_id, after_seq?}` | `{events:[…]}` (journal replay) | pure replay; no live tail |
| `run/subscribe` | `{run_id, after_seq?}` | `{subscription_id, run_id, after_seq}` then `run/event` notifications | replay-then-live; see projection contract below |
| `run/unsubscribe` | `{subscription_id}` | — | |
| `approval/list` | — | `{approvals:[{id,run_id,tool_call_id,decision,expires_at}]}` | durable pending approvals only |
| `approval/respond` | `{approval_id, decision, reason?}` | `{approval_id, decision}` | decision ∈ `approved|denied`; see §7 cancel note |
| `question/list` | — | `{questions:[{id,run_id,tool_call_id,prompt,status,expires_at}]}` | |
| `question/respond` | `{question_id, answer}` | `{question_id, answer}` | empty answer rejected; cancel path is `review/respond` |
| `review/list`, `review/get` | `{review_id?}` | review items `{id,kind:{approval\|question},status,…}` | unified inbox across both kinds |
| `review/respond` | `{review_id, action, answer?, reason?}` | `{review_id,status}` | approval: `approve\|deny`; question: `answer\|cancel` |
| `background/list`, `background/recover`, `background/attach` | — | — | detached-run recovery after window/app restart |

### 2.1 Notification streams

- `run/event` `{subscription_id, event:{run_id,seq,type,created_at,payload_version,payload}}` —
  events keyed `(run_id, seq)`; replay emits journal order then live tail via
  `Bus.Subscribe`; server initiates via `Peer.NotifyContext` (bounded
  writer, cancellation-aware).
- `run/stream_error` `{subscription_id, reason}` — stream terminated abnormally
  (e.g. overflow/transport); the client MUST re-snapshot and re-subscribe.
- `session/work/event` / `session/work/stream_error` — `session/work/subscribe`
  stream for plan/todo work view (buffers commits after captured watermark;
  `work_subscription_test.go` pins watermark semantics).

Projection contract (frontend): install listener → snapshot (`run/log`,
`approval/list`, `question/list`, `session/work/get`) → `run/subscribe` with
`after_seq` = last contiguous seq → merge by `(run_id,seq)` → on
`run/stream_error` or gap, re-snapshot and re-subscribe. This is the algorithm
the `vivy-bridge` event pump implements once, process-wide.

### 2.2 Old `agent-*` event → VIVY event mapping

| Old Tauri event | VIVY `run/event` type (domain/event.go) |
|---|---|
| `agent-response-delta` | `model.delta` |
| `agent-reasoning-delta` | `model.reasoning_delta` |
| `agent-response-complete` | `model.completed` (+ `run.completed` terminal) |
| `agent-tool-start` | `tool.requested` → `tool.started` |
| `agent-tool-delta` | `tool.operation` |
| `agent-tool-end` | `tool.finished` |
| `agent-error` | `run.failed` (+ `run/stream_error` for transport) |
| `agent-provider-retry` | `provider.retry` |
| `agent-provider-stalled` | `provider.stall` |
| `agent-context-compaction` | `context.compacted` |
| `agent-plan-todo-created/updated/completed/cancelled`, `agent-plan-ready`, `agent-plan-report-ready`, `agent-turn-plan-updated` | `session/work/event` family (+ `plan/*` RPCs) — **semantics differ**, §5 |
| `agent-background-response` | `run/event` on detached run via `background/list`+`attach` |
| `approval-event` | `tool.approval_required` / `tool.approval_decided` / `tool.approval_expired` / `tool.approval_cancelled` |
| `approval-stream-connected` | none — subscription ack replaces it |
| `external-message` | `channel.inbound` (listener is dead today — no producer) |
| `desktop-pet-*` (7 events) | none — native multi-window shell ops, §3 native |

Terminal types: `run.completed`, `run.failed`, `run.cancelled`,
`child.completed`, `child.failed`, `child.cancelled` (domain `Terminal()`).

## 3. Disposition ledger

Labels: `existing backend` (verified VIVY RPC/action target), `native`
(shell-owned), `backend gap` (no VIVY surface), `retire pending approval`,
`offline import` (data migration).

### 3.1 Core chat/session — `existing backend` (DN-1)

| Old invoke | VIVY target |
|---|---|
| `send_message` | `turn/start` |
| `stop_generation` | `run/cancel` |
| `get_sessions` | `session/list` |
| `get_session_history` | `session/messages` |
| `delete_session` | `session/delete` |
| `update_session_title` | `session/rename` |
| `list_approvals`, `decide_approval` | `approval/list`, `approval/respond` |
| `list_ask_user_questions`, `answer_ask_user_question`, `cancel_ask_user_question` | `question/list`, `question/respond`, `review/respond {action:"cancel"}` |
| `start_approval_stream`, `start_background_stream` | `run/subscribe` + `approval/list`; `background/*` for detached |
| `check_health` | `initialize` (protocol_version probe) |

### 3.2 Settings/providers/config — `existing backend` (DN-2)

`load_config`/`get_config`/`save_config`/`update_config`/`get_config_status`
→ `settings/get`, `settings/update`. Providers CRUD/probe →
`settings/providers`, `settings/providers/upsert|delete|refresh`,
`settings/model/select`. MCP → `settings/mcp`, `settings/mcp/upsert|delete`,
`settings/mcp/probe`. Channels → `channel/get`, `channel/inspect`,
`channel/update`, `channel/deliveries/*`.

### 3.3 Skills/reviews/cron/tools — `existing backend` (DN-3)

`get_skills`,`get_skill` → `skills/list`,`skills/get`;
`enable/disable` → `skills/set-enabled`; revisions → `skills/revisions/list`;
marketplace → `skills/marketplace/search|featured|install|check`.
Skill-request flow → `review/list|get|respond` (`ReviewKind` domain).
Cron CRUD/enable/run/stop → `cron/list|create|update|delete|trigger|stop`.
Tools config → `tools/list`, `tools/set-active`.

### 3.4 Plan/work surface — `existing backend, semantics differ` (DN-3; design in DN-0 §5)

`get_plans`/`get_plan`/`get_plan_reports`/`get_active_plan_execution`/
`get_execution_todos`/`update_execution_todo`/`approve_active_plan_execution`/
`continue_approved_plan_execution`/`return_active_plan_to_draft`/`delete_plan`
→ `plan/get|enter|leave|decide`, `session/work/get`, `session/work/subscribe`,
`session/todos`, `session/todo/update`, `goal/*`, `deliverables/list|get|read|close`.
Old DIVA ran an *approval workflow for plan executions*; VIVY models plan mode
(enter/decide/leave) plus a work view + goals. Field-level DTO mapping is a
DN-3 design decision, not an assumed 1:1 rename.

### 3.5 Gateway/service/logs lifecycle — `retire pending approval` (DN-1 removes invoke seams)

`start_gateway`, `stop_gateway`, `get_gateway_process_status`,
`get_gateway_status`, `install_service`, `start_service`, `stop_service`,
`uninstall_service`, `uninstall_gateway`, `get_service_status`,
`get_runtime_info`, `tail_logs`, `get_gateway_log_lines`,
`get_gui_log_lines`, `append_gui_log`. The embedded host IS the runtime;
there is no gateway to manage. Bridge init envelope carries identity
(generation_id, protocol_version) that replaced `get_runtime_info`.
`get_audit_events` → **backend gap** (no audit RPC).

### 3.6 Token stats — `existing backend` (DN-2), scope reduce

`get_token_usage_*` (5 commands) → `stats/tokens` with
`{period, tz_offset_minutes, session_limit}` → snapshot. Old per-model/
timeline/realtime split collapses into the single snapshot shape — confirm
required coverage against UI at DN-2 design.

### 3.7 Mask/persona/memory/autodream/evolution — `backend gap` → DN-4 (VIVY-CONTRACT-BLOCKED)

`list_masks`/`get_active_mask`/`get_current_mask`/`switch_mask`/
`create_or_update_mask`/`delete_mask`: VIVY has `internal/maskcontract`
(error codes surfaced through module actions) but **no dedicated mask control
RPC** — mapping via typed actions must be designed in DN-4.
`persona_*` (10), `memory_*` (9), `get_self_evolution_config`,
`save_self_evolution_config`, `trigger_autodream`, `get_autodream_*`,
`cancel_autodream_run`, `list_recall_feedback`: evolution/mask/memory domain
has `generations/*`, `evals/*`, `promotions/*`, `species/inspect` — shape
overlap only; semantic mapping unresolved → DN-4.

### 3.8 Voice/pet — mixed `native` + `backend gap` (DN-6 blocked)

Native shell: `open/close/minimize_desktop_pet`,
`set_desktop_pet_always_on_top`, `set_desktop_pet_ignore_mouse`,
`set_splash_complete`, all `pet_*_vrm_*` model file ops, voice asset file ops.
Backend gap: `pet_siliconflow_synthesize`, `pet_minimax_synthesize`, STT fetch
(`voice-api.ts`), TTS fetch (`tts-service.ts`) — provider calls currently carry
raw API keys in the frontend; P0 requires credentials owned by VIVY → DN-6.

### 3.9 Notebook/history/workspace — `existing backend` partial + `backend gap`

`search_notebook_session_evidence_command` → `history/search`;
`get_notebook_reports`/`trigger_notebook_report_generation` →
`deliverables/*` + history; workspace browsing → `workspace/list|read|browse`.
`get_sandbox_config`/`save_sandbox_config` → `session/set_permission`
(preset) + settings — detailed sandbox fields → gap check at DN-2.
`wipe_local_data` → `native` (shell deletes local dirs) pending owner call.
`get_command_rules`/`set_command_rule_enabled`/`delete_command_rule` +
dead `get_command_approvals`/`resolve_command_approval` → `commands/list`,
`commands/expand` surface — **partially dead already**; resolve-command-
approval path is a backend gap pending policy design.
`list_subagent_results` → `child/list` (was phantom — now real target).
`generate_session_title` → server auto-title at `session/create`; frontend
trigger becomes a re-title request — `session/rename` or **gap** (choose at
DN-1 design). `reset_session` → `session/rewind` or delete+create —
**semantic check required** (rewind is per-message truncation, not a reset).

## 4. ABI v1 freeze (from P0-D1, evidence-bounded)

Exports: `VivyInit`, `VivyCall`, `VivyPollEvents`, `VivyShutdown`, `VivyFree`.
Every non-null return is UTF-8 NUL-terminated, library-allocated; the Rust
side copies then calls `VivyFree` once. No Go pointer crosses FFI.

Envelope: `{ok:true,value:{…}}` | `{ok:false,error:{kind,code,message,data?}}`.
Kinds: `invalid_input, incompatible_abi, closed, already_initialized,
transport_lost, timeout, event_gap, internal`.

Bounds (frozen defaults — all MUST be constants checked at `VivyInit`):
- input frame ≤ 4 MiB (JSON-RPC method+params); `VivyCall` rejects above →
  `invalid_input`.
- event queue 10 000 entries; overflow drops oldest, sets sticky `gap` —
  poll returns `{events:[…], gap:true}`; bridge surfaces `event_gap`.
- `VivyPollEvents` returns ≤ 500 events per call (caller loops on `gap` or
  batch-full).
- `VivyCall` default timeout 120 s → `timeout`; per-call override in params.
- `VivyShutdown` blocks ≤ host `shutdownGrace` (5 s, app.go:64) then returns;
  idempotent; post-shutdown calls → `closed`.
- `VivyInit` second call → `already_initialized`; version word mismatch →
  `incompatible_abi`.
- Header/identity: generated C header MUST define `VIVY_ABI_VERSION 1`; Rust
  asserts `VivyInit`'s returned abi version equals the header constant.

Embedded host wiring (evidence §7): `app.New(ctx, cfg, WithoutEars(),
WithoutGateway())` → `a.DialControl(ctx, notifHandler)` (net.Pipe +
`JSONLTransport`, `internal/app/facehost.go:137`) → hold `*App` for the
process lifetime; quit → `a.Close()` (idempotent, `shutdownGrace`-bounded).
**`App.Run` must not be used as the embedded liveness loop** — §7.

## 5. Session deletion, planning, work review — separate surfaces (plan constraint honored)

- `session/delete` removes the session record; orphan-run policy must be
  confirmed by fixture capture (pending).
- Planning/todos are NOT approvals: `session/todos`, `session/todo/update`,
  `session/work/get|subscribe`, `plan/*`, `goal/*`, `deliverables/*` are a
  distinct surface from `review/*` and `approval/*`. DN-0 found **no** path
  that maps plan approval onto `approval/respond`; the old DIVA plan-approval
  workflow has no VIVY twin — recorded as semantic difference, DN-3 designs
  the mapping.

## 6. Retirements pending approval (front-end invokes with no VIVY equivalent and no shell owner)

`greet` (test seam), plus the 21 dead registrations in §1 and the manager/
service commands in §3.5 that only existed to babysit the old gateway
process. Dead event seams (`desktop-pet-subtitle`, `desktop-pet-emotion`,
`desktop-pet-close-request`, `external-message` listeners; orphaned
`desktop-pet-voice-message`, `desktop-pet-active/-inactive` emitters) are
retirement candidates recorded in TODOLIST `DEAD-INVOKE-SEAMS`.

## 7. VIVY-side evidence and defects found

1. **Gatewayless `App.Run` returns immediately** (`internal/app/app.go:1723-1734`):
   with `httpServer == nil`, `errCh` is closed with no value; the select's
   first case yields `nil` → `return nil`. `Run` never blocks on `ctx` in the
   gatewayless branch. `TestGatewaylessRunWithoutFaceCancelsDurably`
   (`facehost_test.go:352`) never asserts Run was still alive at cancel time —
   the embedded lifetime is unproven, confirming
   `GATEWAYLESS-LIFETIME-EVIDENCE` / DN-L must add the assertion.
2. **Interaction sweeper + cron are Run-scoped**: `StartInteractionSweeper` and
   `StartCronScheduler` are called only inside `App.Run`
   (`app.go:1704-1710`); `New`+`DialControl`+`Close` (the embedded path, as in
   `RunFace`) never starts them → approval/question expiry and cron do not run
   in embedded mode. DN-L must either start them in the bridge init or use a
   composition that owns them; this is a **release-blocking contract item**,
   not an optional enhancement.
3. `run/cancel` returns `CodeNotFound` "run is not active in this process" —
   after an app restart a run may exist in the journal but be unowned; cancel
   of a recovered/listed run must tolerate this (bridge maps to a defined
   outcome, not a crash). Pairs with `background/recover`.
4. `approval/respond` has no `cancel` decision — `review/respond` supports
   question `cancel` but approval actions are `approve|deny` only. Old
   `cancel_approval` semantics → map to `deny` or rely on `expires_at`;
   decision recorded for DN-1 (lean: `deny` with reason `user_cancelled` —
   pending owner call).
5. Eino quarantine verified: only `internal/runtime/` and
   `internal/provider/` import `cloudwego/eino*` (`eino v0.9.13`, eino-ext
   pins in go.mod; `internal/channelhost/deps.go` hit was a comment).
6. `inofy` pin `v0.0.0-20260930141905-71e2c9bbe47d`; Go `1.26.4`.

## 8. Build target / toolchain evidence

- Target OS: Windows (CI `runs-on: windows-latest`, product is `vivy.exe`).
  DLL target: `windows/amd64` via `go build -buildmode=c-shared` →
  `vivy.dll` + generated `vivy.h`.
- Rust host: Tauri v2 + `cargo`/`rustc` present on this box; `cargo` can run
  Rust-side compile checks but cannot produce the Go DLL (no Go toolchain).
- Pack path: `sdk/internal/frontend_v1.go` builds `./cmd/vivy` with
  `go build -p=2 -mod=readonly -overlay …` + sealed manifest
  (`SealManifest`, `inspect` parses `EmbeddedManifest` from the binary). The
  shared-library target must extend the same pack/inspect surface (manifest +
  digest + header) — DN-L must not bypass sealed admission.
- Fixture capture + `go test ./internal/app ./internal/rpc -run
  'LoopbackControl|Gatewayless|Subscribe' -count=1` are **pending**: no Go,
  `just`, or node on this host. Recorded as `P0-NATIVE-VERIFICATION` —
  DN-L stays Blocked until a provisioned host runs them.

## 9. Blockers that keep DN-1/DN-L out of Ready

1. Wire-transcript fixture capture (this doc asserts source schemas only).
2. Gatewayless lifetime + sweeper/cron ownership (§7.1, §7.2) — needs a VIVY
   composition decision: either the embedded bridge runs `App.Run`-equivalent
   startup without the listener, or DN-L ships a dedicated embedded entrypoint.
3. DN-4 domain surfaces (mask/persona/memory/evolution), DN-6 voice
   credentials — `VIVY-CONTRACT-BLOCKED`.
4. Plan/work DTO mapping design (§3.4), `reset_session` semantics,
   `cancel_approval` decision mapping, `generate_session_title` gap call —
   recorded; owners in DN-1/DN-3.
5. c-shared build of `./cmd/vivy`-equivalent entry: no `main`-exportable
   package exists today for the shared target — DN-L adds it and re-pins via
   pack/inspect.

## 10. Amendments applied downstream

- `p0-design.md` ABI section: bounds frozen in §4 are the normative set
  (queue 10 000, poll ≤500, call ≤120 s, shutdown ≤5 s, frame ≤4 MiB).
- `index.md` DN-0 ledger state → see story file; DN-1/DN-L remain
  not-Ready per §9.
- TODOLIST additions: `EMBEDDED-SWEEPER-OWNERSHIP` (sev-P0, from §7.2 —
  new defect class), `RUN-CANCEL-RESTART-NOTFOUND` (P2, §7.3),
  `APPROVAL-CANCEL-VOCAB` (P2, §7.4).
