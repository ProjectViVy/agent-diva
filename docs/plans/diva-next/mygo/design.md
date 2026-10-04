# MyGo observation branch design — MY-D1

## Purpose and authority

Maintain a small, rebuildable MyGo research branch while Wails remains the
delivery path. The user requested a long-lived observation branch and a plan.
The initial publication authorized planning. The owner authorized inline
implementation on 2026-10-04. Scheduled monitoring and a framework switch remain
separate actions.

Choose a thin MyGo host over the same sealed VIVY public SDK and retained DIVA
frontend. Documentation alone is cheaper but cannot answer native feasibility;
maintaining two production shells increases ongoing cost without a current
product need. The chosen branch delivers bounded evidence, not another product.

## Requirements

| ID | Observable outcome |
| --- | --- |
| MY-R1 | One observation ledger identifies exact upstream versions, source evidence, triggers and unresolved questions. |
| MY-R2 | The candidate owns one VIVY Host; page cancellation, hide and reload preserve it; explicit Quit closes it once within the existing budget and reports unfinished teardown truthfully. |
| MY-R3 | Existing Vue/VRM, Agent RPC/events and speech contracts are reused; trusted caller, media, credentials and stale-result protections pass native probes. |
| MY-R4 | The candidate is built and inspected through the existing sealed Go-host pipeline; installer tooling consumes that exact executable. |
| MY-R5 | Comparison records measured behavior and total maintenance/build costs using identified inputs; no invented performance gain or acceptance threshold. |
| MY-R6 | Relevant changes trigger explicit reevaluation; branch upkeep is bounded; promotion requires an owner decision after evidence review. |

## Baseline and dependency authority

DIVA main: `5444795a2d9db31e158c2cf009d64697e6289e50`.
VIVY main: `fc559e6b03ce4e65c0099b9745855dccc4fb067e`.
The [source ledger](observations.md) records upstream MyGo evidence.

The existing [DN-W3 design](https://github.com/ProjectViVy/agent-diva/blob/1e9dca1aed557009f5d868c79e3c907d963ede42/docs/plans/diva-next/p0-design.md#dn-w3-go-owned-desktop-migration-2026-10-04)
and [W3 contract ledger](https://github.com/ProjectViVy/agent-diva/blob/1e9dca1aed557009f5d868c79e3c907d963ede42/docs/plans/diva-next/backend-separation-contracts.md#w3-go-host-contracts)
remain authoritative for public Host, RPC, event, speech and sealing semantics.
Those Go interfaces/build facilities are proposed at the inspected baseline.
The research branch does not implement missing mainline facilities.

Before MY-1 product implementation, integrate the accepted Wails/Go-host baseline
into this branch, reconcile repository rules/source drift and record its exact
commit. MY-0 is a standalone framework probe and does not wait for this input.
Retain observation history. Do not continuously merge every main commit.
Use isolated checkouts for the Wails baseline and MyGo candidate; never start
both against one profile/data root. Fresh disposable profiles contain no legacy
DIVA import, production credential copy or user-data migration.

## Boundaries and proposed files

All Go paths below are proposed at this baseline.

| Surface | Responsibility |
| --- | --- |
| `cmd/diva-mygo/main.go` | Research entrypoint; main-thread App.Run; register bindings before Run, acquire native windows/Host only after app readiness. |
| `internal/desktop/mygo/owner.go` | One public SDK Host and one active Next reader; admission, cancellation and bounded teardown. |
| `internal/desktop/mygo/app.go` | MyGo window, single-instance, native caller and quit callbacks; no Agent logic. |
| `internal/desktop/mygo/bindings.go` | CallReply DTO normalization and page-scoped Channel subscription. |
| `internal/desktop/mygo/media_http.go` | MyGo protocol/native-sender adaptation over accepted Go speech service. |
| `agent-diva-gui/src/platform/mygo-host.ts` | Framework client calls, cancellation, channels and media routes. |
| `agent-diva-gui/src/platform/desktop-host.ts` | Retained public facade; research-branch replacement of framework implementation. |
| `agent-diva-gui/src/generated/mygo/client.ts` | Generated client; generated code is not hand maintained. |
| `scripts/build-desktop.py` | Accepted W2 wrapper; bounded research extension to target `./cmd/diva-mygo`. |

Business stores, Vue components, VRM assets and provider implementations remain
shared with the accepted baseline. Reuse `internal/speech` after W4. A small
private MyGo lifetime adapter is necessary; do not copy VIVY's runtime or introduce
a framework registry, second daemon, Rust bridge or native-Go UI rewrite.
Changes required to shared public contracts leave this plan's scope.

## Runtime and communication contracts

Consume `agent-vivy/sdk/host/v1` exactly as W3-1 specifies:
`Open(context.Context, host.Options) (*host.Host, error)`,
`Host.Call(context.Context, string, json.RawMessage) (json.RawMessage, error)`,
`Host.Next(context.Context, int) (host.EventBatch, error)`, and
`Host.Close(context.Context) error`. No external import of VIVY internals.

Private proposed MyGo types/signatures:

- `NewOwner(options host.Options) *Owner`
- `Owner.Start(ctx context.Context) error`
- `Owner.Call(ctx context.Context, request CallRequest) CallReply`
- `Owner.Subscribe(ctx context.Context, out *mygo.Channel[WireEvent]) error`
- `Owner.Shutdown(ctx context.Context) error`
- bound facade `RuntimeBindings.Call(ctx context.Context, request CallRequest) CallReply`
- bound facade `RuntimeBindings.Subscribe(ctx context.Context, out *mygo.Channel[WireEvent]) error`

CallRequest/CallReply mirror W3-2 JSON fields, including typed numeric errors.
WireEvent mirrors the existing TS union: `{kind:"vivy",method,params}` or
`{kind:"bridge",status:"gap"|"lost"}`. The TS factory
`createMygoTransport(): VivyTransport` preserves call/onEvent/close semantics.
No generated framework type escapes into business stores.

The application owns a root context; page-injected contexts only own calls and
subscriptions. Subscribe acquires a single reader slot, returning the normalized
already_initialized error for duplicate admission. It drains Next batches of up
to 500 and sends the same wire events through MyGo Channel. Page cancellation
unblocks Next/Send and releases the reader; it never closes Host. The Host's
10,000-event bounded queue/gap contract absorbs a slow or absent page without
stalling runtime producers. Gap triggers authoritative refetch; never replay audio.

Window close means hide, invalidate voice context and stop browser audio. Reopen
resubscribes and reads current projections. Privileged bindings accept only the
verified primary CallerWindow; external navigation/secondary windows receive no
privileged capability. Single-instance admission precedes Host.Open.

On explicit Quit, prevent the initial native quit, close admission and run the
single shutdown coordinator off the UI thread with an independently owned
five-second context. Cancel the subscription, await Host.Close, record its result,
then allow one native Quit through a guard. On timeout report unclean exit;
do not reopen another Host or claim leaked work was killed. OnQuit does not start
another teardown. Partial startup releases acquired resources without ready.

## Speech/native trust and build constraints

Consume W3-3 unchanged: Go provider HTTP, OS keyring, capability-bound raw audio,
2 KiB speech metadata, 10 MiB/file, 20 assets, 100 MiB aggregate and 128-byte
display names. Preserve keyring namespace `dev.projectivy.diva.speech`, config
CAS, cancellation and identity/generation checks. Keys never return to Vue.
The generic MyGo fetch plugin is outside the candidate's privileged speech path.

MyGo protocol requests expose native window identity via framework-assigned
RemoteAddr; CallerWindow applies to bound-method contexts, not arbitrary HTTP
contexts. MY-0/MY-2 must prove cross-window and navigation behavior before using
that metadata. JS headers/origin/window IDs are never proof of native identity.

Pin the initial candidate to MyGo v0.2.4 / commit
`154449f603ede999a1dfc8692717de62f7bacf12`; its go.mod requires Go 1.27.1.
The research compiler is isolated and recorded. Do not silently raise the Wails
delivery toolchain (currently 1.26.4). Framework no-CGO claims do not prove
the full VIVY/speech dependency closure is CGO-free.

Generate bindings with staged dependency/assembly inputs and verify that
MYGO_GENERATE cannot initialize Host or native services. Build frontend once,
then SDK `pack --target go-host` with the MyGo entrypoint, supplied assets and
source lock; Inspect the result. No `mygo build` or installer task may compile
a replacement unsealed executable. Research installers use a separate experimental
application identity/profile and never replace the Wails installed product.

## Observation and decision policy

Manual triggers: an upstream fix/API change affects quit, IPC/cancellation, media,
WebView2, packaging or supported toolchains; accepted mainline Host/speech changes
affect the adapter; or an operator requests another comparison. A trigger creates
a ledger row first. Cosmetic/native-toolkit changes need no full test rerun.

Record version/input pins, affected rows, migration diff, commands, actual results,
native environment, missing evidence, costs and the recommendation. Changed
framework inputs invalidate corresponding prior passes; retain historical runs.
No calendar schedule, background monitor or automatic issue creation is implied.

A switch is eligible only after installed Windows parity and native trust/lifetime
checks pass, public contracts stay intact, and measured total cost benefits justify
migration and maintenance. The owner chooses observe, pause/drop, or commission a
separate migration plan. This research plan never merges/promotes automatically.
Missing native machines/credentials remain pending; local mock success does not
replace configured-provider or owner acceptance.

## Verification and economy

Four sequential Stories are sufficient; [index](index.md) owns their status/DAG.
Reuse baseline tests and fixtures; add only adapter/lifecycle regressions.
Comparison reports raw startup/memory/build/runtime observations where measurable,
method and variance; toolchain differences are stated and compiler-controlled
runs are separate. Include maintained code, dependency/generation changes,
packaging friction, actual upkeep effort and upstream patch responsibility.
There is no claimed speedup, LOC quota or date commitment.
