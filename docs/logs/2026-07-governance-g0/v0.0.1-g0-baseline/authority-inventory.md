# G0 Authority and Capability Inventory

Unknown capabilities, sources, states, and scopes are denied.

| Domain | Read path | Side-effect path | Authority owner | Final policy decision | Event / recovery | Scope and replay policy |
| --- | --- | --- | --- | --- | --- | --- |
| Applied Memory/Persona | `MemoryProvider` startup, recall, prompt rendering | approved proposal applied by `LaputaService` | `agent-diva-laputa` applied sections | proposal validation and receipt before apply | changelog, audit, snapshot reload | workspace + section + content hash; rollback through changelog |
| Legacy Memory | `MemoryManager` compatibility reads | legacy sync only without Laputa | legacy core memory boundary | `default_memory_provider` selects one authority | workspace-file reload | workspace; no silent fallback after Laputa failure |
| Mentle | search/status and prompt adapter | retrieval-index maintenance | Mentle retrieval runtime, never authority | activation and capability filter | rebuildable index; explicit degraded state | workspace index; no authority writes |
| AutoDream | sessions and applied snapshots | proposals and reports | AutoDream artifacts; Laputa applied authority | proposal boundary | run/proposal/report stores | run ID; inference never promotes itself |
| Plan | report/plan snapshot | approval, materialization, execution | canonical planning/report store | phase policy plus expected revision | persisted events and restart restoration | plan ID + revision; stale receipt rejected |
| Sandbox | policy/rule lookup | command execution | `agent-diva-sandbox` | Guardian/orchestrator before execution/retry | pending store, SSE, decision, cancellation | command hash + cwd + resource scope; once consumed |
| Manager | service projections | calls owning service | domain service remains authority | server validation/authorization | typed HTTP/SSE and durable stores | session/workspace/resource; no second truth |
| Tauri/GUI | Manager projection or declared local state | submits decisions/local commands | Manager domain; Tauri LOCAL host capability | Manager or local command guard | reconnect/reconcile | GUI visibility is not authorization |
| Tools/MCP/spawn/schedule | registered metadata | AgentLoop execution seam | owning tool/provider plus sandbox | assembly and pre-call policy | tool events and supervised store | declared resource; unknown denied |

Invariants: evidence producers only propose; AgentLoop only orchestrates; Manager/GUI
only project and submit; durable authority and pending decisions restore; every
production side effect maps to one owner and one final decision point above.
