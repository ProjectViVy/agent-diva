# G0 Authority and Capability Inventory

This inventory freezes ownership and decision points before G1 changes runtime seams.
Unknown capabilities, sources, states, and scopes are denied.

| Domain / capability | Read path | Write or side-effect path | Authority owner | Policy decision point | Event / recovery path | Scope | Idempotency / rollback | Default / evidence / approval TTL |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Applied Memory and Persona | `MemoryProvider` startup, prefetch, and prompt rendering | Approved proposal applied by `LaputaService` | `agent-diva-laputa` applied sections | proposal validation plus approval before apply | Laputa changelog, audit events, snapshot reload | workspace + section + proposal hash | apply is content-bound; changelog supports eligible rollback | deny unknown source; proposal evidence required; receipt expires before apply |
| Legacy Memory fallback | `MemoryManager` reads legacy workspace files | legacy sync only when Laputa is not configured | `agent-diva-core` legacy memory boundary | `default_memory_provider` selects exactly one authority boundary | workspace files reload on startup | workspace | file operation semantics; no durable approval receipt | allowed only as explicit compatibility path |
| Mentle recall | Mentle search/status tools and prompt adapter | index maintenance only; never Laputa authority writes | Mentle runtime owns retrieval index, not authority | feature/runtime activation plus tool capability filter | runtime rebuild; failure degrades without authority fallback | workspace index | rebuildable index; no authority rollback | deny prompt/tool exposure when runtime or status capability is absent |
| AutoDream | reads sessions and applied Laputa snapshots | emits proposals and reports | AutoDream owns generated artifacts; Laputa owns applied authority | proposal submission boundary | run store, proposal state, report logs | workspace + run | run IDs prevent accidental reuse; proposal can be rejected | generated inference is untrusted evidence, never direct authority |
| Plan execution | planning/report snapshots and active-plan projection | revision-bound approve, materialize, execute, transition | planning store owns revision and execution state | phase policy plus expected revision/receipt | persisted report/plan events and restart restoration | plan ID + revision | CAS rejects stale approval; terminal state is durable | unknown phase/capability denied; approval bound to revision |
| Sandbox command execution | policy and validated-rule lookup | orchestrator executes command | `agent-diva-sandbox` owns execution policy and approval cache/store | Guardian/orchestrator immediately before execution and retry | pending request store, SSE, decision record, cancellation | command hash + cwd + environment/resource scope | once receipt is consumed; session/rule grants are explicitly scoped and revocable | deny unknown/risky action; approval expires with request/session policy |
| Manager HTTP/SSE | reads service projections | invokes owning service APIs | domain service/store remains authority; Manager is transport | server-side validation and authorization | typed HTTP errors, SSE replay/reconnect, persistent stores | authenticated session/workspace/resource | mutation idempotency belongs to domain contract | UI visibility never authorizes an action |
| Tauri/GUI | renders Manager projection or local host state | submits decisions or local host commands | Manager for domain state; Tauri only for declared LOCAL capabilities | Manager for domain action; Tauri command guard for host action | reconnect/reconcile from authoritative projection | GUI session + selected workspace | duplicate UI events are reconciled; domain receipt controls replay | DEFERRED/REMOVED capabilities have no transport |
| Filesystem/network/MCP/spawn/schedule | registered tool metadata | AgentLoop tool execution seam | owning tool/provider plus sandbox policy | tool assembly filter and pre-call policy | tool events, audit correlation, supervised run store | declared path/host/process/schedule scope | per-tool idempotency; irreversible actions require stronger policy | unregistered or unmapped capability denied |

## Production path invariants

1. Evidence producers may propose but cannot promote their output to authority.
2. AgentLoop orchestrates; it does not become a second Plan, Memory, Sandbox, or approval store.
3. Manager and GUI project state and submit commands; neither may maintain a competing domain truth.
4. A restart reloads durable authority and pending decisions. Ephemeral session grants do not silently become durable.
5. Every production side effect must map to one owner and one final policy decision point in this table before G1.
