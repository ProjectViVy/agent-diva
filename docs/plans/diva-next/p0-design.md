# DIVA two-P0 delivery design — review draft

Revision: P0-D1. This is the concrete planning response to the user's Tauri thin-shell + Go shared-library directive. It supersedes the conflicting transport/host choices in DN-P1 for these P0s; it does not claim external issue approval or implementation approval. The [index](index.md) alone owns delivery state and dependencies.

## Outcomes and baseline

- P0-A: ship a DIVA-specific sealed VIVY shared library, a small Rust `vivy-bridge`, and a Tauri shell. Prove chat → streamed answer → real approval → cancellation → close/reopen window recovery → explicit application exit.
- P0-B: account for every legacy runtime call/event and replace its behavior, state authority and event semantics with verified VIVY RPC/actions or explicitly native operations. Renaming commands, mocks, hidden failures, or disabling required features do not satisfy this outcome.
- P0-A can close before P0-B. Neither closure implies full DIVA Next release, speech parity, companion parity or historical import unless its own acceptance explicitly includes them.
- Planning baselines: DIVA `d96e396d1641a5e7636e30e1f6cdbd0e597b62c8`; VIVY `5347032d8f18a047b67e85761c0dcc48728bee7c`. These are inspected source revisions, not accepted binary pins. DN-0 pins the integration inputs and DN-L records the produced artifact hash.
- Observed DIVA runtime scan: 159 invoke sites in 17 files, excluding tests/stories; 81 in desktop.ts; 26 listen sites in 3 runtime files. Counts are baseline evidence, not hard-coded acceptance totals; DN-0 must enumerate aliases/wrappers too.
- Windows x64 is a planning candidate, not an already approved platform matrix. DN-0 records the required target and actual runner/toolchain availability before native work becomes Ready. No calendar duration or staffing capacity has been supplied; scheduling uses dependency waves rather than invented dates.

## Architecture decisions

Vue presentation → typed VIVY client → one Tauri bridge command (`vivy_call`) and event `vivy:event` → Rust `vivy-bridge` → C ABI → Go embedded host → existing DialControl/JSON-RPC → the existing VIVY Service.Run, Journal, policy and action host.

The Tauri application, not a window, owns one Go runtime. Closing the main window hides/destroys presentation while the application remains alive in the tray; reopening reconstructs state from VIVY. Explicit Quit starts bounded shutdown. Rust owns windows, tray, native dialogs and ABI memory copies; it has no agent loop, domain database, provider credentials or business-command translation table. VIVY owns authoritative sessions, runs, approvals, settings, credentials and domain persistence. Native appearance preferences are the exception.

Use an in-process control peer and polling across FFI. This reuses existing control contracts and avoids a gateway listener, local authentication endpoint and cross-language callbacks. `WithEventSink` is not a second UI authority: notifications come through the control peer after subscriptions. No local HTTP/WS transport or old Manager fallback is added for desktop. A browser preview must explicitly report desktop unavailable; unit fixtures never masquerade as a live backend.

Shared-library support must remain in the existing Recipe → generated Assembly → sealed Generation build path. `sdk/internal/frontend_v1.go` currently builds `./cmd/vivy` and inspects a platform executable; merely running `go build -buildmode=c-shared` is insufficient product evidence. Extend this same pack/inspect path with an explicit shared target; preserve executable-default compatibility. Do not hand-edit generated assembly or create a second plugin compiler. A DIVA recipe selects only the modules justified by DN-0; domain additions are coordinated with their owning Stories, not guessed from the headless recipe.

Go `1.26.4`, Eino `v0.9.13`, and the INOFY pin in the inspected go.mod are baseline inputs. No new Eino orchestration, runtime or provider abstraction is planned. Before implementation, VIVY workers read its AGENTS.md, vivy-plugin and (for compiler/host changes) vivy-kernel-ci guidance. Document the pinned capability check: reuse current runtime streaming/checkpoints/policy through control RPC; keep Eino imports inside the existing quarantine.

## Proposed ABI v1 (not existing exports)

```c
char *VivyInit(const char *options_json);
char *VivyCall(unsigned long long handle, const char *request_json);
char *VivyPollEvents(unsigned long long handle, unsigned int wait_ms);
char *VivyShutdown(unsigned long long handle);
void VivyFree(char *owned_json);
```

Every non-null return is a UTF-8, NUL-terminated allocation made by the library; Rust copies it immediately and calls VivyFree exactly once. No Go pointer crosses FFI. Null means allocation/FFI failure and is not valid JSON. Inputs are borrowed only for the synchronous call and copied before asynchronous use. Embedded NUL and invalid JSON/UTF-8 are rejected. The generated cgo header is the source for exact platform declarations; ABI width assertions are required. No dynamic unload/reload of the Go library during process lifetime.

Init input: `{abi_version:1,data_dir:absolutePath,settings_path:absolutePath}`. Paths point to a new test/product data root, never production data in verification. Output envelope: `{ok:true,value:{handle,abi_version:1,generation_id,protocol_version}}` or `{ok:false,error:{kind,code,message,data?}}`. Handle is serialized as a decimal string to JavaScript; Rust/Go use a 64-bit integer. One successful initialization per process; repeated identical Init returns the same running instance, conflicting options fail `already_initialized`, and Init after Shutdown fails `closed`. Partial initialization cleans up before reporting failure.

Call input: `{request_id:string,method:string,params:object|null}`. It calls the existing peer.Call with the method and parameters and returns the same request_id with the success/error envelope. Keep VIVY RPC error code/data intact under `kind:rpc`; bridge failures use `kind:bridge`. Bridge error codes include `invalid_input`, `incompatible_abi`, `closed`, `already_initialized`, `transport_lost`, `timeout`, `event_gap`, `internal`. The request_id correlates responses; it is not a new backend idempotency key. Unknown mutation outcome is shown as unknown and reconciled through reads, never auto-resubmitted. A call timeout does not mean run cancellation; cancellation is `run/cancel` with the authoritative run ID.

Poll output value: `{events:[{method,params}],gap:boolean}`. Preserve backend notification payloads, sequence numbers and order. One process-owned Rust poller drains a bounded Go queue; queue limit and poll limits are frozen by DN-0 as tunable bounds with boundary tests, not claimed performance targets. On overflow, set a sticky gap flag, discard the unusable queued suffix, and require snapshot/replay before any UI cursor advances. The poller must not block the control reader or UI thread. Rust republishes events on `vivy:event`; a missing window is harmless because reopening re-reads/replays durable state. Transient events that are not journaled are reconstructed from current snapshots or explicitly classified in DN-0.

Shutdown rejects new calls, wakes polling, stops work, drains/cancels runs while the Journal is open, closes peers before storage, and returns the recorded result on repetition. In-flight calls must resolve or fail before shared resources are closed. A bounded shutdown failure is reported and the owning application exits; it must not unload a still-running Go runtime or kill another process. The exact deadline uses the existing application shutdown policy after DN-0 verifies all close operations are bounded.

## Lifecycle and projection algorithm

```text
startup: validate ABI/artifact → open one embedded host → register event drain → initialize RPC
window open: attach event listener first → read sessions/runs/pending interactions
             → subscribe after last contiguous sequence → merge buffered events by (run_id, seq)
             → fetch run/log or re-read snapshot on a gap → expose connected projection
window close: detach presentation only; never close App/Journal or cancel unrelated runs
explicit cancel: run/cancel → await authoritative terminal state
quit: reject new submissions → stop/drain runtime → close peer/storage → exit shell
```

Keep backend IDs and review revisions unchanged. Deduplicate persisted replay by backend run ID/sequence; never infer approval from frontend state. Completed-only answers must work without deltas. Pending approvals/questions must survive window reopen and process restart. Two windows share the same runtime and do not independently submit or decide the same action automatically.

Lifecycle evidence gap: App.Run currently closes errCh in the gatewayless branch and can return immediately; the existing cancellation test does not assert that Run remains alive before cancellation. DN-L must add that assertion and choose the minimal correction to the embedded lifetime without changing listener-mode semantics. Do not use `go App.Run(); defer App.Close()` as an assumed working ownership model.

## Scope and acceptance

| Requirement | Responsible Stories | Required evidence |
| --- | --- | --- |
| A1 sealed DLL/header/Recipe with identity | DN-0, DN-L | pack + inspect + tamper rejection + C caller smoke |
| A2 thin shell and process lifetime | DN-5 | window reopen retains runtime; duplicate launch/data lock; bounded exit |
| A3 real chat/stream/approval/cancel/recovery | DN-1, DN-2, DN-P | deterministic full chain and live-model packaged smoke |
| B1 exhaustive semantic mapping | DN-0 | all source consumers/events/data owners accounted for |
| B2 state/event and domain replacement | DN-1/2/3/4/6 | per-domain accepted behavior, errors and restart evidence |
| B3 no retired runtime seams reachable | DN-M | AST/import/call graph gate + package inspection + ledger reconciliation |

P0-B cannot close with a required mapping marked blocked, merely hidden or mock-backed. Explicit retirement requires the user's scope decision and a record; the plan does not authorize it. Missing backend capabilities stay domain-specific blockers, not a reason to block the independent core loop. Data import remains DN-7 and full release remains DN-8.

## Review focus and measurement

Most important adverse cases: duplicate/late events; window close during approval; cancel racing with completion; timeout after an accepted mutation; shutdown while FFI polling/calls are active. Each has a test owner in the Story plans. Measure stream lag, queue occupancy, startup and shutdown during native acceptance; no unmeasured SLA or speedup is promised. Known execution limitation: this environment has no Go command; no new Go/native/runtime verification is claimed by this plan.

## OBS-D1 investigation update (2026-10-02)

The environment-only “no Go command” observation above is historical: official Go 1.26.4 has now been installed locally and Eino v0.9.13 downloaded for source inspection. This is not a native runner, accepted ABI or Generation. The [observability extension](observability/architecture.md) supplies concrete DN-0/DN-3 log, call, usage and trajectory requirements against VIVY 8fc6bea; the selected binary pin remains DN-0/DN-L's responsibility. P0-D1 host/transport decisions are unchanged.
