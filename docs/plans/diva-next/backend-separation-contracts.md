# DN-C2 closure contracts — 2026-10-03

Architecture: [DN-C2](p0-design.md); status/dependencies: [index.md](index.md).
This section is the current ledger. **Existing** means inspected source at
DIVA f5866a0 / VIVY 1db8b55 / Laputa dc6066e; selected artifact/runtime capture
is separate. **Proposed** means designed here, not exported or Ready.
Historical inventories below remain evidence only at their recorded pins.
Do not edit captured `fixtures/core-rpc.json` to pretend new contracts ran.

## C2-1. Existing transport and producer contracts

| Surface | Existing wire / behavior | Consumer obligation |
| --- | --- | --- |
| C ABI init | `{abi_version,config_path,without_ears?}` -> `{abi_version,handle}` | Numeric handle stays native; generated header/hash proof |
| C ABI call | `{method,params,timeout_ms?}` -> `{ok:true,value}` or `{ok:false,error:{kind,code,message,data?}}` | Timeout is not rollback; no replacement request_id ABI |
| C ABI poll | Handle + uint32 max-events; five C exports including Free | Copy/free once; gap/refetch; no wait_ms argument |
| Module action | `module.action.invoke` params `{module_id,action_id,input}` | Sealed definitions/grants, authenticated caller and host authorization |
| Turn images | `turn/start` attachments `[{name?,mime_type,data}]`; data is base64 image bytes | Total call <=4 MiB including expansion/framing; reject unsupported files/model |
| Session permission | `session/set_permission {session_id,preset}`; preset cautious/smart/trusted; returns session DTO | Confirm/read back before send; admitted policy remains authoritative |
| Rewind | `session/rewind {session_id,message_id}` -> cutoff/remaining result | Inclusive cutoff; append-only underlying Journal; busy -> conflict |
| Atomic text edit | `session/edit {session_id,message_id,text,mode?,face?,policy_profile?,thinking?,collaboration_mode?,collaboration_version?}` -> `{run_id,status}` | No attachments field at inspected pin; only text regeneration uses this path |
| Run cancel | `run/cancel {run_id}` -> cancelling; inactive process run -> not-found | Read run/session to distinguish already terminal from stale/unavailable |
| Trajectory | `trajectory/session {session_id,limit?}` -> v2 DTO | Default 20 runs, max 50; one projection owner |
| Diagnostics | `diagnostics/logs` and `diagnostics/gui/append` | Bounded owned families; preserve gaps/truncation/partial ambiguity |
| Token stats | `stats/tokens {period?,tz_offset_minutes?,session_limit?}` -> projection v2 and coverage | Source is chat_runs; request_count counts reports, not all billed requests |
| Skills history | `skills/revisions/list` when backing dependency is bound | Capability proof required; upload/edit/delete/request remain residuals |

Current ABI guards: 4 MiB input, event queue 10,000 with oldest-drop gap,
poll <=500, default call timeout 120 s, shutdown grace 5 s. No incompatible
ABI change for cognition or native audio. JSON integers consumed by Vue must
be validated as safe integers; do not silently round revisions/sequences.

Actual console DTOs, rather than PR shorthand:

```typescript
// Existing field names; detail row DTOs mirror owning Go structs.
type TrajectorySession = {
  session_id: string; projection_version: 2;
  records: TrajectoryRecord[]; requests: TrajectoryRequest[];
  run_activity: TrajectoryRunActivity[];
  watermarks: Record<string, number>; has_older_runs: boolean;
};
type DiagnosticQuery = {
  source: 'runtime' | 'gui'; date?: string; after?: string;
  limit?: number; level?: string; query?: string;
};
type DiagnosticPage = {
  source: string; records: DiagnosticRecord[];
  next_cursor?: string; gap: boolean; has_more: boolean;
};
```

Trajectory request rows retain legacy fields but v2 UI reads `call_status`,
`finished_at`, `usage_state`, `usage_evidence`. Call states:
active/completed/failed/cancelled/interrupted/legacy. Usage:
missing/reported/partial/active/legacy. `usage_evidence` is nullable;
reasoning/cached buckets may be absent. Run activity distinguishes
queued/active/waiting/completed/failed/cancelled and authoritative wait_kind.
Use actual `request_id`, `run_id`, `call_id` and stable record IDs, not array
indices as identities.

Diagnostic records: `id,at?,level?,component?,message,fields?,truncated`.
GUI append: `{records:[{at?,level?,component?,message,fields?}]}` ->
`{accepted:number}`. Source limit 500 records, 8 KiB per record, 4 MiB scan/batch.
Frontend batches are deliberately smaller (<=50 records and <=256 KiB JSON)
to leave ABI framing headroom. Current partial-write RPC error describes
accepted prefix in text; it is not a structured dedupe receipt. Do not retry
an ambiguous append blindly.

Token coverage fields: state empty/complete/partial/legacy, observed_calls,
completed_with_usage, reported_calls, missing_usage_calls, partial_usage_calls,
active_calls, legacy_usage_records, unknown_buckets, hidden_retries_observable.
Observed calls partition into completed-with-usage/partial/missing/active;
reported_calls is not another disjoint partition. `total.cost_known=false`
makes numeric cost a placeholder, not “free.” Do not claim hidden provider
retry/billing observability.

Regenerate design is now fixed: atomic `session/edit` for text turns; for image
turns, preserve original bytes, validate before mutation, then inclusive rewind
at originating user message and normal `turn/start` with original images.
Busy/failed/ambiguous stages reconcile; never silently convert image to text.
A rewind succeeded/send failed state retains retryable draft and is visible.

### DN-0C closure capture — source-pinned semantics

Fixture: `fixtures/closure-chat-obs.json` (`capture_kind: source_host`,
`vivy_pin: 1db8b55ce905ee4a212326e7801e0958e7f4376a`, `protocol_version:
vivy.rpc.v1`, transport `App.DialControl` net.Pipe JSONL). Producer:
`internal/app/dn0_capture_test.go::TestDN0CaptureClosureTranscript`, run with
`DN0_CAPTURE_OUT=<path> go test ./internal/app -run
TestDN0CaptureClosureTranscript -count=1 -v`. Historical
`fixtures/core-rpc.json` and `TestDN0CaptureCoreTranscript` are unchanged.

Exact symbols/fields confirmed by the capture (VIVY pin above):

| Contract | Source symbol | Frozen semantics |
| --- | --- | --- |
| Error codes | `internal/rpc/protocol.go` + `control.go:48` | `InvalidParams=-32602`, `InternalError=-32603`, `CodeNotFound=-32004`, `CodeConflict=-32009` |
| Permission preset | `controlHandler.setSessionPermission` | enum error `InvalidParams` "preset must be cautious, smart, or trusted" |
| Attachment decode | `startTurn` param decode | bad base64 → `InvalidParams` "attachment N: data must be base64-encoded image bytes"; unsupported MIME → `InvalidParams` "unsupported type … (png, jpeg, gif, webp)" |
| Busy session | `Service.rejectBusySession` | `session/rewind` during an active run → `CodeConflict` "runtime: session has an active run" |
| Inclusive rewind | `Service.RewindSession` | `RewindResult{cutoff_message_id,remaining_count}`; cutoff message itself is removed (`remaining_count` counts strictly-before rows); missing message → `CodeNotFound` "cutoff message is not in the live session view" |
| Atomic edit | `editSession`/`Service.EditSession` | `{run_id,status:"accepted"}` starts one run and replaces the message suffix; missing message_id → `InternalError` (-32603), not a typed not-found |
| Cancel/reopen | `cancelRun` | `run/cancel` on a terminal run → `CodeNotFound` "run is not active in this process"; there is no reopen method — a fresh `turn/start` after a cancelled run is the admitted reopen path |
| Fork | `forkSession` | `{session_id,fork_point_message_id,copied_count}`; copy is inclusive of the fork-point message |
| Trajectory | `sessionTrajectory`/`Service.SessionTrajectory` | unknown session_id → empty projection (v2, `records:[]`), not an error; `watermarks` values are safe-integer run sequences |
| Token stats | `statsTokens` | `projection_version:2` plus `coverage{active_calls,completed_with_usage,hidden_retries_observable,…}`; invalid `period` → `InvalidParams` "invalid period: <value>" |
| Diagnostics | `diagnostics/gui/append` + `diagnostics/logs` | >500-record batch → `InvalidParams` "batch exceeds 500 records"; unknown `source` → `InvalidParams`; appended GUI records round-trip with server-assigned `id`,`at` |
| Child reads | `child/list`,`child/get` | `child/list{parent_run_id}` → `{children:[]}`; missing run → `CodeNotFound` "child run not found" |
| Plan→Goal | `plan/decide` action `start_goal` | decide *resumes* the interrupted plan run with a reviewer tool-result, then the Goal driver admits a separate round run; only the admitted run may `report_goal` ("only the admitted Goal run may report Goal state") |
| Attachment replay | `session/messages` | `attachments[].data_url = data:<mime>;base64,<bytes>`; bytes are byte-identical to the sent payload; `include_attachment_data` defaults true |
| Work projection | `session/work/get` | `{version, plan{active,review_status,submission_id,origin_run_id}, goal{phase,…}}`; `plan/enter`→`turn/start`→`plan/decide`→`plan/leave` is the full captured lifecycle |

## C2-2. Proposed same-owner library ports

Reuse `agentapi.Open` and public DTOs. Exact new Go API names are proposals,
with the following required signatures/semantics to freeze in owning library:

```go
// PROPOSED capability shape, not existing exported code.
type EvolutionPorts struct {
    Domain evolution.Domain
    SourceID string
    HighWatermark func(context.Context) (uint64, error)
    MissionRevision func(context.Context) (uint64, error)
}
```

These ports are constructed inside Garden from its one owned runtime, fixed
to admitted scope/destination. Owner Close invalidates all derived capabilities;
they never close shared authority independently. A public configuration
extension selects a Backend/factory using existing `garden/memory.Backend`.
It is mandatory for full memory capability; no implicit writer fallback.
All retrieval/ingestion/effects use that selection. First supported adapter
is existing Garden Mentle, explicitly selected in fresh setup, with no implicit
BML replacement. Missing local embedding
capability is explicit degradation; no implicit model download.

Trusted VIVY composition validates a session against its own registry before
asking the same client for session/workspace/principal-specific capability.
The library preserves fixed profile and admitted scopes. Human operations are
never available through the agent capability; public API presence itself is
not runtime authorization. No `garden/internal` import by VIVY.

VIVY factory provides these contract seams:

| Seam | Input / output | Lifetime |
| --- | --- | --- |
| Primary prepare | Host session/run/workspace + budget -> FrozenCore v2 reference and bounded authority text | Before inference; immutable session snapshot |
| Optional recall | Existing ContextSource query -> scoped bounded cards/evidence | Required profile bind; optional retrieval failure reported |
| Capture sink | Existing CognitiveCapture -> existing receipt seq/status | Stable committed terminal identity, no supervisor input |
| Strategy binding resolver | Fixed host domain + current authority -> existing RunBinding | Per admission; persist once and verify on recovery |
| Run-bound domain guard | Persisted RunBinding -> existing Domain | Before effects; no authority refresh of admitted run |
| Control resolver | Server-authenticated invocation origin/session -> human facade | No JSON authority or generic tool entry |
| Close | No input | Stop observers/Service uses before closing one Garden owner |

The generated observer inventory includes the existing cognitive capture
provider identity and its current terminal event/field policies. Domain sink
is armed before observer recovery; wake callback can attach afterward. Never
append an unsealed observer subscription as an app-only exception.

## C2-3. Proposed cognitive action inventory

Owner/module ID: `vivy/diva-cognitive`. Names below are intentionally separate
from ordinary `vivy.memory.*` BML actions. Existing `module.action.invoke`
carries the result; no second transport. Inputs are strict closed schemas:
reject unknown/duplicate fields, invalid enum/trailing JSON/unsafe numbers.
Each includes `session_id` for target correlation, verified against the
transport-bound session and trusted registry; it grants no authority.
Before Persona setup the host can create a setup session without starting a
primary run, so the UI can obtain an authenticated bound control context.

All these actions are human control-plane actions, never model tools. Agent
reads/mutations use separately compiled scoped tools and the same domain
capability. Definitions remain schema/grant/Generation validated.

| Action suffix (prefix `diva.cognitive.`) | Input beyond session_id | Successful value / owning operation |
| --- | --- | --- |
| `status` | none | CapabilityStatus below |
| `persona.initialize` | `initialization` (existing typed Initialization), `reason` | Existing WriteOutcome/readiness; trusted human actor stamped by adapter |
| `persona.read` | `kind` | Current authorized PersonaDocument; closed eight-kind roster |
| `persona.save` | `kind,content,base_revision,reason` | Existing SaveUserDocument CAS; Mission allowed only through human path |
| `persona.reviews.list` | `kind?,state?,limit?,cursor?` | Bounded existing ChangeRequest page; state pending/accepted/rejected/stale |
| `persona.review.decide` | `review_id,decision:accept|reject` | Existing domain outcome; stale base never forced |
| `frozen.read` | none | Existing FrozenCore v2 for target session, or explicit not-yet-captured |
| `actmem.read` | `sections,max_chars` | Existing scoped ActivityResult; inherited projection caps |
| `actmem.work.patch` | `patch` (existing WorkPatch) | Existing ActivityResult / revision conflict |
| `actmem.owner.read` | none | Human whole-document Markdown/revision, never agent projection |
| `actmem.owner.save` | `markdown,base_revision` | Validated whole-save result; parser/caps enforced |
| `memory.search` | `query,collection?,cursor?,limit,budget_chars` | Existing CardPage from selected backend, admitted read-scope union |
| `memory.expand` | `card_id,expected_revision,budget_chars` | Existing EvidencePage; record scope/revision verified |
| `memory.mutate` | `mutation` | Existing AuthorizedMutation payload minus scope/destination; host stamps fixed writer, normalized digest and operation identity |
| `memory.receipt` | `operation_id` | Existing scoped MutationReceipt lookup |
| `policy.get` | none | Durable TriggerPolicy and policy_revision |
| `policy.set` | `enabled,min_interval_ms,base_revision` | Durable policy CAS, not racing automatic admission |
| `trigger` | none | Existing Eligibility plus actual active_run_id/window; blocked unknown remains blocked |
| `cancel` | `run_id` | Validated current active strategy run; existing cancellation plus durable pause |
| `results.list` | `cursor?,limit?` | Bounded reflection/effect receipts from existing domain ledger |

Page limit defaults 20, maximum 100, with no hidden-scope totals. Cursor is
opaque and bound to scope/query/revision; invalidation is explicit. Action
input <=64 KiB, output <=256 KiB unless a tighter owning domain cap applies.
Owner ACTMEM caps and Persona projection rules remain unchanged. No generic
unlimited Markdown reader is admitted.

The policy CAS revision is a safe numeric UI revision. Existing RunBinding
policy_revision remains a string pin derived from that revision plus normalized
policy digest; it is not relabelled as a numeric domain field.

New policy revision/CAS and result paging are adapter extensions; current
runtime UpdateCognitivePolicy/CognitiveStatus alone do not provide them.
Do not claim those inputs are already accepted. Memory mutation returns the
host-assigned operation identity for receipt lookup; if a transport timeout
hides that identity, outcome remains unknown, and it is not resubmitted.
Human CAS save ambiguity reads current authority/history; identical current
content alone does not prove which operation committed.

Proposed business-result envelope (ActionHost still owns auth/schema/runtime
errors):

```typescript
type CognitiveOutcome<T> =
  | { status: 'ok'; value: T }
  | { status: 'unavailable' | 'failed' | 'unknown';
      error: { code: string; message: string; retryable: boolean };
      value?: T }; // only safe recovery receipt/state, never hidden authority

type CapabilityStatus = {
  profile_id: string;
  scope: Scope; destination_id: string;
  persona: { state: string; current_revisions: Record<PersonaKind, number> };
  frozen: { state: 'not_captured' | 'ready' | 'recovery_required';
            revisions?: Record<FrozenKind, number> };
  memory: { backend_id: string; health: 'available' | 'degraded' | 'unavailable';
            reason_code: string; canonical_state: string; index_state: string };
  cognition: { enabled: boolean; policy_revision: number; min_interval_ms: number;
    eligibility?: Eligibility; active_run_id: string; source_id: string;
    watermark: number; pending_through: number;
    phase: 'disabled' | 'idle' | 'running' | 'paused' | 'recovery_required';
    block_reason: string };
};
```

Domain DTOs Scope, WorkPatch, ActivityResult, FrozenCore, AuthorizedMutation /
MutationReceipt, EffectReceipt and ChangeRequest map to owning library types;
do not build a second domain DTO dialect. Guarded aliases are typed in Vue.
`status:ok` only means action succeeded; EffectReceipt submitted is still not
applied, and an asynchronous ingestion accepted is not completed.

Stable domain codes are preserved (including revision/scope/conflict/Mission /
ACTMEM/backend/recovery codes), not raw provider error strings. If ActionHost
or ABI timeout/panic loses business result, read operations may retry explicitly;
writes become unknown until authoritative reconciliation. `retryable` never
means automatic replay of an ambiguous write. No generic force-clear-recovery
endpoint exists; unsafe state remains blocked pending authoritative resolution.

Proposed request example, not captured execution:

```json
{
  "method": "module.action.invoke",
  "params": {
    "module_id": "vivy/diva-cognitive",
    "action_id": "diva.cognitive.persona.save",
    "input": {
      "session_id": "session-demo",
      "kind": "mission",
      "content": "An explicitly human-authored mission.",
      "base_revision": 3,
      "reason": "Owner edit"
    }
  }
}
```

It contains no actor/profile/scope/approval claim. Transport origin and domain
human capability decide authorization; ordinary Agent full-auto permission
does not grant Mission ownership.

## C2-4. Proposed native speech command contract

Frontend native seam: `src/platform/desktop-host.ts` only. Register exact
commands in shell, mirrored in AST/backend-boundary gates. Use
`#[tauri::command(rename_all = "snake_case")]` consistently for JSON command
arguments. They never multiplex arbitrary URLs/methods/paths/provider commands.
Main window only; caller label obtained natively. Plain browser mode reports
native-unavailable, not a browser provider fallback.

```typescript
type SpeechIdentity = {
  request_id: string; session_id: string; run_id?: string;
  utterance_id: string; generation: number;
};
type SpeechFailure = {
  identity?: SpeechIdentity;
  code: 'native_unavailable' | 'not_configured' | 'device_unavailable'
    | 'credential_unavailable' | 'provider_error' | 'invalid_audio'
    | 'unsupported_reference' | 'asset_not_found' | 'stale_context'
    | 'invalid_input' | 'revision_conflict' | 'busy' | 'cancelled' | 'timeout';
  message: string; retryable: boolean;
  provider?: 'siliconflow' | 'minimax'; http_status?: number;
};
```

No SpeechFailure includes secret/audio/text/body/URL. Native rejects unsafe
integers, unknown fields, invalid IDs/enums/lengths. generation is monotonic
per native caller window for this host lifetime; controller reload obtains
current generation in config_get, increments, then context_set. ID/generation
is correlation/fencing, not a VIVY authority grant.

| Command | Input | Success / rule |
| --- | --- | --- |
| `speech_config_get` | none | Versioned preferences/revision, credential presence/availability, current window context; no keys |
| `speech_config_update` | `{base_revision,preferences}` | Updated safe config; full closed preferences schema and native CAS |
| `speech_credential_set` | `{provider,key}` | `{provider,present:true}` after OS store success; no key echo |
| `speech_credential_delete` | `{provider}` | `{provider,present:false}`; absent deletion idempotent |
| `speech_context_set` | `{session_id,generation}` | Same context; strictly newer generation cancels old requests; identical current tuple idempotent, older/conflicting tuple rejects |
| `speech_transcribe` | Raw WAV bytes; bounded metadata header described below | `{identity,status:'transcribed',text}` or `{identity,status:'no_speech',text:''}` |
| `speech_synthesize` | `{identity,text}` | Raw MP3 ArrayBuffer through ipc::Response; native config selects provider/model/voice |
| `speech_cancel` | `{request_id}` | `{request_id,status:'cancelled'|'settled'}`; window-scoped, does not claim remote refund |
| `voice_asset_import` | Raw reference bytes + bounded metadata header | Asset descriptor; native ID and owned copy |
| `voice_asset_list` | none | <=20 descriptors; no filesystem paths |
| `voice_asset_read` | `{asset_id}` | Raw bytes for explicit local preview; known descriptor MIME |
| `voice_asset_delete` | `{asset_id}` | `{asset_id,status:'deleted'|'pending'}`; read lease release completes pending delete |

Raw transcribe invoke example (**proposed**, not tested on bundled artifact):

```typescript
invoke('speech_transcribe', wavArrayBuffer, {
  headers: { 'x-diva-speech-meta': JSON.stringify({
    identity, mime_type: 'audio/wav'
  }) }
});
```

Native accepts only Raw body, metadata <=2 KiB, valid identity, WAV PCM16 /
mono/16kHz and <=120 s/8 MiB. Headers are IPC metadata, never provider
Authorization. Reference import header `x-diva-asset-meta` carries only
`{display_name,mime_type}` <=2 KiB; verify WAV/MP3 bytes, not extension alone.
No source path/remote URL. Native derives safe filename from ID; caller name
is display-only. Read/delete unknown ID is asset_not_found, never arbitrary
path access. Manifest writes use atomic file replacement under native lock.

Preference schema `diva.speech/v1`:

- `stt: {provider:'siliconflow',base_url,model}`.
- `tts: {provider:'siliconflow'|'minimax',siliconflow:{base_url,model,voice,
  speed,reference},minimax:{base_url,model,voice_id,speed,volume}}`.
- SiliconFlow reference is a strict union: system; reusable `{voice_id}`;
  inline `{asset_id,transcript}`. Exact provider/model incompatibility rejects,
  never drops configured reference. No speculative MiniMax cloning workflow.
- `auto_read_replies:boolean`. Runtime bounds are fixed native guards, not
  client-controlled timeout/size overrides. No audio translation option.
- `credential_state` and `window_context` are native readback only; neither
  can be written in preference payload. No PetConfig/localStorage speech key.

Credentials keyed by fixed app/profile/provider. Provider HTTPS origin is
configured by human settings, never per utterance. Validate no URL credentials /
fragment/query; no cross-host redirects or automatic retries. Each operation
freezes config revision. Release native request buffers on every path; Vue
owns MP3 Blob URL and revokes it. **No speech_release_audio command or persistent
native TTS cache.**

Errors use rejected invoke Promise with SpeechFailure. Successful binary TTS
contains no custom framing; identity remains tied to the call's captured
Promise and current generation. JSON STT echoes identity. Cancel/context changes
always stop local playback first; late binary response is discarded/revoked.
New playback never drains a stale queued answer.

**DN-0S evidence (2026-10-03):** source-pinned compatibility record in
[`fixtures/closure-speech.json`](fixtures/closure-speech.json). Pinned
versions/features after proof: `tauri 2.12.1` (lockfile), `keyring 4.2.0`
features `v1` + platform stores (`windows-native-keyring-store` compiles for
x86_64-pc-windows-msvc; Linux stores map unavailable to
`credential_unavailable`), `reqwest 0.13` minimal TLS set `default-features =
false, features = ["rustls", "webpki-roots", "json", "multipart"]` — HTTPS to
`api.siliconflow.cn` proven (HTTP 401 = TLS path good); `rustls` pulls the
`aws-lc-rs` provider so Windows builds require MSVC + cmake. IPC Raw/Response
seam verified at pinned source level plus Windows-target `cargo check`; live
webview roundtrip, Windows WebView2 audio, OS store success and authenticated
provider calls stay pending for DN-6/owner acceptance.

## C2-5. Contract proof required before Ready

- Capture existing selected-artifact image/edit/rewind/permission/OBS fixtures;
  keep proposed examples labelled and separate from captured core transcript.
- Implement public facade conformance and sealed factory/provider inventory
  checks, origin rejection and required FrozenCore-to-model-input evidence.
- Prove no supervisor/child capture, stable receipt replay, source namespace,
  per-run Mission fence and no new-attempt replay after unknown outcome.
- Probe bundled Tauri Raw/Response, WebView WAV/MP3, credential OS persistence /
  unavailability, native memory/response caps and abort race.
- Refresh provider region/model/reference request fixtures, especially MiniMax
  whose current documentation fetch timed out. No real provider success or
  quality claim is made by this ledger.
- Amend existing native/AST/dependency gates for exact commands; activated
  speech is removed from the broad dormant pet exemption. Negative fixtures
  reject browser cloud fetch, dynamic/generic native invoke and old pet_*.

**DN-0P evidence (2026-10-03):** the pinned build and native-dependency
inventory is captured in
[`fixtures/closure-build-inputs.json`](fixtures/closure-build-inputs.json) —
exact VIVY/Laputa(garden, laputa, mentle dirs)/INOFY commits plus directory
tree hashes, toolchain, full transitive replace map, module graph captures,
CGO/system-library inventory (libc only; no ONNX/native tokenizer/OpenSSL/
libsqlite3 in the baseline closure), staged pack/inspect commands, baseline
artifact SHA-256s, and the Windows/amd64 build+load command recorded pending
(no windows/amd64 runner, no mingw-w64 cross compiler).


## C2-6. Execution-plan producer seams (proposed)

Added by the 2026-10-03 Story decomposition. These names resolve implementation
handoffs within DN-C2; they are **not exported source, captured runtime proof,
or a change to domain/wire authority**. DN-4A/DN-LC/DN-4B freeze actual
implementations and fixtures here before dependent plans become Ready.

### Garden public ownership and controls — DN-4A

Extend `agentapi.Config` with `BackendID string`, `DestinationID string`,
and `Backends map[string]memory.Backend`. The configured writer is explicit;
`mentle` selects the existing adapter, with no second backend on failure.
Validate exact fixed subject/write scope/destination and resource paths at
Open. The owner may derive admitted personal/workspace read scope from trusted
VIVY session metadata; wire payloads cannot issue that admission.

Proposed Go surface in `garden/agentapi/embedded_domain.go`:

```go
func (c *Client) BindHumanSession(sessionID, workspaceID string) (*HumanClient, error)
func (c *Client) BindAgentSession(sessionID, workspaceID string) (*BoundClient, error)
func (c *Client) BindEvolution(scope evolution.Scope, destinationID string) (EvolutionPorts, error)
func (h *HumanClient) InitializePersona(ctx context.Context, in persona.Initialization, reason string) (*persona.WriteOutcome, error)
func (h *HumanClient) SavePersona(ctx context.Context, kind persona.Kind, content string, baseRevision uint64, reason string) (*persona.WriteOutcome, error)
func (h *HumanClient) ListPersonaReviews(ctx context.Context, q ReviewQuery) (ReviewPage, error)
func (h *HumanClient) DecidePersonaReview(ctx context.Context, id string, decision ReviewDecision) (*persona.WriteOutcome, error)
func (h *HumanClient) SaveACTMEM(ctx context.Context, markdown string, baseRevision uint64) (ActmemDocument, error)
func (h *HumanClient) SearchMemory(ctx context.Context, q memory.AuthorizedSearch) (memory.CardPage, error)
func (h *HumanClient) ExpandMemory(ctx context.Context, q memory.AuthorizedExpansion) (memory.EvidencePage, error)
func (h *HumanClient) MutateMemory(ctx context.Context, m memory.AuthorizedMutation) (memory.MutationReceipt, error)
func (h *HumanClient) MemoryReceipt(ctx context.Context, operationID string) (memory.MutationReceipt, error)
func (h *HumanClient) ReadFrozen(ctx context.Context) (FrozenCore, error)
func (h *HumanClient) Results(ctx context.Context, q PageQuery) (ResultPage, error)
```

HumanClient is available only from a trusted `PrincipalUser` owner. Derived
BoundClient stamps reduced `PrincipalAgent` per handle instead of reusing
the owner's principal. Existing BindSession keeps its existing behavior for
other consumers. Per-handle principal/scope is private host-issued state; it
cannot be set in a read/write request. ReadPersona/ReadActivity/ReadACTMEM/
ApplyWorkPatch remain typed public methods on the appropriate bound handle.
All methods take the same owner's read/lifetime lock; derived capabilities
never reopen or Close a second authority.

`EvolutionPorts` is C2-2's shape, constructed internally with the already-open
Persona/ACTMEM/ingest/backend and `garden/evolution.NewDomain`; source identity
includes profile/scope/destination. Domain reads/effects use exactly that
scope and the same host authority gate used by Mission human writes.

That host gate belongs to the selected VIVY bundle: both the human-write
adapter and run-bound effect wrapper acquire it before entering Garden's
lifetime/domain locks. Library operations never call back into the host
while holding those locks. This preserves one serialized check/write boundary
without exporting a library mutex or holding a global lock over inference.

Pagination structs live beside the public facade, not a new database:

- `PageQuery`: `Cursor string`, `Limit int`; default 20/max 100.
- `ReviewQuery`: optional `Kind *persona.Kind`, `State *persona.RequestState`,
  plus PageQuery. `ReviewPage`: `Items []persona.ChangeRequest`, `NextCursor string`.
- `ReviewDecision`: closed `accept|reject`. Call existing AcceptRequest/
  RejectRequest; stale base is never forced.
- `ResultPage`: bounded receipt/reflection projections of existing
  `effects.jsonl` and `notes.jsonl`, plus `NextCursor string`. Each item retains
  operation_id/payload_digest/status/kind/target_ref/revision/error_code;
  reflection text/sources/at are returned only for an owned note.

Cursors encode an opaque scope/query/ledger-version checkpoint. Scope/version
change invalidates them; no hidden-scope totals. Memory methods validate the
host-stamped Authorized DTO against the admitted reader/writer, never trust
wire-provided scope/destination/operation identity. The adapter issues those
fields before calling MutateMemory and returns the operation identity.
Actor/source/reason policy uses trusted domain rules; Mission has no agent
write path. Owner ACTMEM save reuses validated parser/CAS/caps.

### Generated bundle and primary preparation — DN-LC / DN-4B

Core-only `internal/cognitivecontract/ports.go` defines the narrow contracts.
It must not import optional Garden implementation or `internal/runtime`.
Move existing CognitiveCapture/Receipt/Source/Sink shapes there if necessary
and keep aliases in runtime; do not duplicate or reinterpret their fields.
The selected factory lives in `internal/modules/diva-cognitive/` and imports
public Garden APIs. Core receives interfaces and existing Laputa DTOs.

Proposed seams:

| Seam | Fixed signature / contract |
| --- | --- |
| Assembly lookup | `CognitiveFactoryValue() any`; App strictly checks `cognitivecontract.Factory` |
| Factory | `Factory(ctx context.Context, input FactoryInput) (Bundle, error)`; FactoryInput contains existing trusted config, Generation and owned storage dependencies |
| Primary input | `PrimaryContextInput{SessionID domain.SessionID, RunID domain.RunID, WorkspaceID string, BudgetBytes int}` |
| Primary prepare | `Prepare(context.Context, PrimaryContextInput) (PreparedPrimaryContext, error)` |
| Primary output | `PreparedPrimaryContext{Frozen evolution.FrozenCoreV2, Digest string, Text string}`; no new snapshot store |
| Per-admission binding | `ResolveBinding(context.Context) (evolution.RunBinding, error)`; source scope/destination fixed |
| Effect/recovery guard | `BoundDomain(context.Context, evolution.RunBinding) (evolution.Domain, error)`; persisted binding, shared authority gate |
| Bundle callbacks | `AttachRuntime(ControlPort) error`; single attachment to generated binding cells, not a new provider registry |
| Bundle close | `Close() error`; after Service/observer admission stops; no derived close |

FactoryInput's data is reused from config.Config/storage.Engine and existing
host constructor dependencies. The enabling plan decides names/lifetime, not
a parallel storage DTO scheme. The same generated adapters obtain this bundle;
primary/sink/source/domain ports are armed before recovery, runtime callbacks
after Service. Selected missing/type-mismatched/unarmed bindings fail init.

ControlPort owns `GetState`, `SetPolicyCAS`, `Trigger`, `Cancel`. It uses the
existing TriggerPolicy/Eligibility/RunBinding and new narrow control projection
for phase/block/policy_revision/window. DN-4B adds:

```go
func (s *Service) CognitiveControlState(ctx context.Context) (cognitivecontract.ControlState, error)
func (s *Service) UpdateCognitivePolicyCAS(ctx context.Context, policy evolution.TriggerPolicy, baseRevision uint64) (cognitivecontract.ControlState, error)
func (s *Service) CancelCognitive(ctx context.Context, runID domain.RunID) (cognitivecontract.ControlState, error)
```

`ControlState` fields map C2-3 CapabilityStatus.cognition verbatim:
enabled/min_interval_ms/policy_revision/eligibility/active_run_id/source_id/
watermark/pending_through/phase/block_reason. No second scheduler/policy store.
Numeric policy_revision is UI CAS; admitted RunBinding.policy_revision remains
its string digest pin. Default policy disabled. Up to three proven-safe
transient attempts; unknown/cancelled/authority-changed work does not auto-retry.

DN-4C extends DialControl with a trusted host option while keeping existing
two-argument calls valid. Only embedded.Host requests embedded-human origin;
ActionHost authenticates its opaque peer and verifies session/registry plus
exact action ID. Origin is private server state, never an RPC field, Face
string alone, or model-tool capability. Human-origin checks do not bypass
ordinary grants or domain restrictions.

### Consumer handoffs

- DN-2A/B own typed chat methods, retained originating bytes and local
  `invalidateConversation(reason)` notification. OBS-07 reads the same event
  owner; it never calls subscribeRun independently.
- OBS-06/07/08 share one `src/api/vivy/observability.ts`. Only the parent index
  coordinates shared-file order. OBS-08 recorder max 1000 rows/1 MiB queued;
  each call max 50/256 KiB. These are bounded implementation guards, not measured
  throughput. Errors/partial accepted-prefix uncertainty are not blindly retried.
- DN-6A/B own C2-4 native command/diagnostic contracts and asset read leases.
  DN-6C alone owns frontend media/Blob URLs and conversation generation.
- New producer proof updates this ledger and downstream plans together;
  illustrative code never becomes a captured fixture by renaming its label.

---

## Historical DN-C1 amendment and initial inventory

The following records are superseded where DN-C2 explicitly changes them.

# DN-C1 contract-ledger amendment — 2026-10-03

Current scope: [DN-C1](p0-design.md); stage state: [index.md](index.md).
The initial inventory below is preserved as a historical baseline. Its old
producer-gap/disposition conclusions are superseded by this amendment;
exact existing core ABI bounds remain authoritative unless changed by evidence.
New cognitive/native-speech IDs are proposed, not verified wire contracts.

| Surface | Current verified source mapping | Remaining contract work |
| --- | --- | --- |
| C ABI init | VIVY cmd/vivy-shared/exports.go: abi_version, config_path, optional without_ears | Generated header and selected artifact proof |
| C ABI call / poll | method, params, optional timeout_ms; poll max-events | No request_id/data_dir/wait-ms replacement ABI |
| Image attachments | VIVY turn/start.attachments exists | Replace DIVA FileAttachmentDto IDs with bytes/MIME; frame bound; explicit unsupported-file error |
| Permission / regeneration | session/set_permission and session/rewind exist | Freeze mappings/selected-message semantics and mutation recovery |
| Skill history | skills/revisions/list exists when backing dependency is bound | Verify compiled capability; upload/delete/edit/request still unresolved |
| Console | trajectory/session projection v2, diagnostics/logs, diagnostics/gui/append, stats/tokens coverage | DIVA typed DTOs and one subscription/readback owner |
| Cognition | Laputa #2 domain libraries; VIVY #26 CognitiveBinding/status/policy/trigger/capture | Public facade, compiled binding, human/domain action fixtures, primary-run context and embedded start/stop |
| Online speech | DIVA wrappers for SiliconFlow STT and SiliconFlow/MiniMax TTS | Tauri native request/config/credential/media/cancel contracts; not a missing VIVY-module dependency |
| Legacy data import | Owner cancelled | No replacement/importer; DN-7 removed from release predecessors |

Inspected pins: DIVA f5866a0, VIVY 1db8b55, Laputa dc6066e. Source availability
does not establish selected Generation capability or real-cloud acceptance.

---

## Historical initial inventory (below)

# Backend Separation Contracts — DN-0 Ledger (proposed)

Status: **PROPOSED — pending owner review**. This document freezes the DN-0
inventory and the ABI v1 contract record. `fixtures/core-rpc.json` is a **captured
transcript**: recorded on a Linux host via an uncommitted `internal/app/
dn0_capture_test.go` harness in the VIVY pin checkout (`DialControl` + gateway-less
composition + scripted DeepSeek-shaped provider), 38 request/response frames +
18 `run/event` notifications; redacted.

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
-32001` plus domain `CodeNotFound = -32004`, `CodeConflict = -32009`
(wire-verified: `question/respond` unknown id → `-32004 "runtime: question
not found"`; `run/cancel` on inactive run → `-32004 "run is not active in
this process"` — fixture records 52, 54).

| Method | Params (source-verified) | Result | Errors / notes |
|---|---|---|---|
| `initialize` | none | `{protocol_version:"vivy.rpc.v1", capabilities:[...], code_mode_available}` | capabilities array is the face's contract-discrimination source |
| `session/create` | `{title, workspace_path}` | `{id,title,created_at,updated_at,sandbox_mode,approval_policy,permission_preset,workspace_path}` | empty title → server-side auto-titler marks untitled |
| `session/list` | — | `{sessions:[sessionResult…]}` | |
| `session/get` | `{session_id}` | `{session:{…}, messages:[messageResult…]}` — returns metadata AND reconciled message history | wire-verified (fixture record 8) |
| `session/messages` | `{session_id, include_attachment_data?}` | `{messages:[messageResult…]}` | reconciles + applies view truncations before returning |
| `session/delete` | `{session_id}` | `{deleted:true}` | wire-verified (record 56) |
| `session/rename` | `{session_id,title}` | — | |
| `session/rewind`, `session/fork`, `session/edit`, `session/context`, `session/compactions`, `session/sidebar`, `session/set_permission`, `session/set_workspace`, `session/todos`, `session/todo/update` | per-handler | — | planning/work surface, §5 |
| `turn/start` | `{session_id, text, mode?, attachments?, attachment_paths?, context_paths?}` | `{run_id, status:"accepted"}` | `RunAccepted` — accepted≠started; terminal arrives via events |
| `run/get` | `{run_id}` | run record `{…,status}` | statuses include `cancelled` |
| `run/cancel` | `{run_id}` | `{run_id,status:"cancelling"}` | `-32004` if run not active in this process — **restart window race**, see §7 |
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
`deliverables/*` + history; workspace browsing → `workspace/browse`
(`{path}` host dirs) + `workspace/list|read` (**`{run_id}`-scoped** per-run
workspaces — old notebook session browsing must carry a run, or use
`workspace/browse`; shape differs from old session-scoped file list).
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
- Fixture capture + `go test -tags vivy_headless ./internal/app -run
  'LoopbackControl|Gatewayless|DN0Capture' -count=1` **executed** on Go 1.26.8
  linux/amd64 (toolchain at `~/toolchains/go`); transcript committed as
  `fixtures/core-rpc.json`. Remaining pending: Windows `c-shared` build,
  header/FFI verification, packaged acceptance (`P0-NATIVE-VERIFICATION`,
  native half only).

## 9. Blockers that keep DN-1/DN-L out of Ready

1. ~~Wire-transcript fixture capture~~ — **done**: fixture is a captured
   transcript; `go test -tags vivy_headless ./internal/app -run
   'LoopbackControl|Gatewayless|DN0Capture' -count=1` passed on Go 1.26.8
   linux/amd64 (`internal/app` ok 0.398s; `internal/rpc` no matching tests —
   subscribe coverage lives in `internal/app` + `work_subscription_test.go`).
2. Gatewayless lifetime + sweeper/cron ownership (§7.1, §7.2) — needs a VIVY
   composition decision: either the embedded bridge runs `App.Run`-equivalent
   startup without the listener, or DN-L ships a dedicated embedded
   entrypoint. The gatewayless tests above do **not** cover this.
3. Windows `-buildmode=c-shared` build + Rust FFI verification host — the
   captured evidence is Linux-only; `windows/amd64` DLL + header + packaged
   acceptance remain unverified (`P0-NATIVE-VERIFICATION` narrows to the
   native half).
4. DN-4 domain surfaces (mask/persona/memory/evolution), DN-6 voice
   credentials — `VIVY-CONTRACT-BLOCKED`.
5. Plan/work DTO mapping design (§3.4), `reset_session` semantics,
   `cancel_approval` decision mapping, `generate_session_title` gap call —
   recorded; owners in DN-1/DN-3.
6. c-shared entry point: no `main`-exportable package exists today for the
   shared target — DN-L adds it and re-pins via pack/inspect.

## 10. Amendments applied downstream

- `p0-design.md` ABI section: bounds frozen in §4 are the normative set
  (queue 10 000, poll ≤500, call ≤120 s, shutdown ≤5 s, frame ≤4 MiB).
- `index.md` DN-0 ledger state → see story file; DN-1/DN-L remain
  not-Ready per §9.
- TODOLIST additions: `EMBEDDED-SWEEPER-OWNERSHIP` (sev-P0, from §7.2 —
  new defect class), `RUN-CANCEL-RESTART-NOTFOUND` (P2, §7.3),
  `APPROVAL-CANCEL-VOCAB` (P2, §7.4).
