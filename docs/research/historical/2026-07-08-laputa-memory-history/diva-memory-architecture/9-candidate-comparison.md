# Agent-Diva Memory Architecture — 9 Candidate Evaluation

Codex subagent report. Read-only investigation. No recommendation provided.
All claims cite `file:line`. Scoring is on evidence only — pre-existing user
disfavor of B2/B3/C3 does **not** affect the score.

---

## Fact-check (confirmations and corrections)

| # | Claim in context | Verified | Evidence |
|---|---|---|---|
| 1 | Rust workspace at `agent-diva/` | ✅ | `agent-diva/Cargo.toml:1-19` (16 members listed) |
| 2 | `LaputaService` reads/writes 14 JSON sections, no shell-out | ✅ | `agent-diva-laputa/src/service.rs:204` iterates `LaputaSectionName::all_v1()`; no `Command::new`; `agent-diva-laputa/src/layout.rs:87-104` reads `.laputa/sections/<name>.json` |
| 3 | `MemoryProvider` trait at `provider.rs:384-415` with 4 steps | ✅ | `provider.rs:384` `pub trait MemoryProvider: Send + Sync`; methods `system_prompt_block:389`, `prefetch:399`, `sync_turn:407`, `on_session_end:413` |
| 4 | `MemoryManager` at `manager.rs` is file-based MEMORY/HISTORY | ✅ | `manager.rs:30` `MemoryManager::new`; `manager.rs:46-58` `load_memory/save_memory`; `manager.rs:67-76` `load_history/append_history` |
| 5 | `HybridMemoryProvider` feature-gated on `mentle` | ✅ | `agent-diva-core/Cargo.toml:60-62` `[features] mentle = ["dep:memtle"]`; `agent-diva-core/src/memory/hybrid.rs:1-4` docstring |
| 6 | `MemtleToolkit` exposes 22 public methods | ✅ | `toolkit.rs:27-210` — confirmed 22 methods: `open:29`, `open_configured:47`, `connection:60`, `tool_definitions:68`, `call_json:74`, `status:88`, `search:93`, `add_drawer:98`, `get_drawer:106`, `list_drawers:114`, `kg_query:122`, `kg_add:130`, `kg_invalidate:135`, `diary_write:143`, `diary_read:151`, `traverse:159`, `find_tunnels:167`, `graph_stats:175`, `create_tunnel:180`, `list_tunnels:188`, `delete_tunnels:196`, `follow_tunnels:204` |
| 7 | Hybrid invokes 3 typed + 1 JSON-dispatched in production | ✅ | `hybrid.rs:30-31` `toolkit.status()` + `toolkit.graph_stats()` (snapshot fetch); `hybrid.rs:388` `toolkit.search(args)` in `prefetch`; `hybrid.rs:447-451` `toolkit.call_json("memtle_diary_write", ...)` in `sync_turn` |
| 8 | `add_drawer` typed call is test-only | ✅ | `hybrid.rs:682` only invocation is inside `mod tests` |
| 9 | `diary_write()` typed method never called | ✅ | `grep diary_write` across `agent-diva/` finds only the JSON-dispatched variant `memtle_diary_write` at `hybrid.rs:449`; typed `toolkit.diary_write()` has no production caller |
| 10 | `LaputaMemoryProvider` at `memory_provider.rs:91` | ✅ | `memory_provider.rs:90-141` confirms `impl MemoryProvider for LaputaMemoryProvider`; `LaputaMemoryProvider::open:27`, `:new:31`, `render_authority_block:44`, `authority_sections:64` |
| 11 | `DegradedMemoryProvider` at `memory_boundary.rs:38` | ✅ | `memory_boundary.rs:38-99` — struct at `:38`, `impl MemoryProvider` at `:57` |
| 12 | Cargo dep direction `agent-diva-laputa → agent-diva-core` | ✅ | `agent-diva-laputa/Cargo.toml:12` `agent-diva-core = { path = "../agent-diva-core" }`; `agent-diva-core/Cargo.toml` does not list `agent-diva-laputa` |
| 13 | Provider selection at `memory_boundary.rs:16-35` | ✅ | `memory_boundary.rs:16` `pub(crate) fn default_memory_provider(workspace: &Path) -> Arc<dyn MemoryProvider>`; chooses `LaputaMemoryProvider` if `.laputa/` present, else `MemoryManager`, else `DegradedMemoryProvider` |
| 14 | Mentle wiring at `mentle_runtime.rs:73-75` | ✅ | `mentle_runtime.rs:73-75` `let memory_provider: Arc<dyn MemoryProvider> = Arc::new(agent_diva_core::memory::HybridMemoryProvider::new(file_manager, toolkit.clone()).await)` |
| 15 | Two disjoint provider paths, never composed into single `Arc<dyn MemoryProvider>` slot at `agent_loop.rs:143` | ✅ | `agent_loop.rs:143` field `memory_provider: Arc<dyn MemoryProvider>`; set from `default_memory_provider` at `:317`; mentle overrides only when `active_memory_provider.is_none()` at `:490-492` — so when `.laputa/` exists and `LaputaMemoryProvider` is chosen, mentle never wins |
| 16 | External Go `laputa.exe` exists but is NOT in diva's runtime | ✅ | `C:/Users/Administrator/Desktop/projects/laputa/laputa.exe` exists; zero references to `laputa.exe` from diva (`grep -r laputa.exe agent-diva/` returns 0 matches) |

### Corrections / clarifications

- **F1**: Context says HybridMemoryProvider uses `mentle = 0.1.2`. The
  workspace pin is `memtle = "0.1.2"` in `agent-diva/Cargo.toml:89` (workspace
  dependency). `agent-diva-core/Cargo.toml:56-58` consumes it via
  `[dependencies.memtle] workspace = true optional = true` — correct.
- **F2**: Context says "calls 1 JSON-dispatched call (`call_json('memtle_diary_write')` in sync_turn)". Confirmed — `hybrid.rs:447-451`. The typed
  `toolkit.diary_write()` method (`toolkit.rs:143`) is defined and accessible
  but **never called from production code**. This is the gap candidate α
  would close.
- **F3**: Context claims `agent-diva-core/memory` has `provider.rs`, `manager.rs`, `hybrid.rs`, `storage.rs`, `mod.rs`. Confirmed: `mod.rs:6-9` lists `hybrid/manager/provider/storage` as `pub mod`.
- **F4**: Context says agent-diva-agent has `agent_loop.rs:143`. Confirmed.
- **F5**: I could not find any **direct** `laputa.exe` shelling inside
  `agent-diva/`. Context claim stands: LaputaService is fully Rust-native.
  External Go `laputa.exe` at `projects/laputa/` is independent.

### Incorrect facts flagged

- **None material.** All eight verified factual claims hold. One minor
  precision: the `memtle = 0.1.2` pin is a workspace-level dependency
  (`agent-diva/Cargo.toml:89`), not a per-crate direct dependency — but this
  is what the context's wording implies ("intentionally pinned to the
  published `memtle = 0.1.2` crate").

---

## Candidate B2 — Go mid-fusion (already disfavored)

> Build a Go binary that internally embeds both the laputa 14-section store
> and mempalace-style hybrid search (BM25 + ONNX vector). Single Go process,
> single binary, single data dir `~/.laputa/`. ~3,500-4,500 lines Go.

**Source code paths (read-only):**
- `C:/Users/Administrator/Desktop/projects/laputa/` — current Go laputa, 3,182 LoC Go total (`.go`), no SQLite/FTS5 backend — `internal/store/redis/store.go` only.
- `C:/Users/Administrator/Desktop/projects/mempalace-go-redis-v2/` — Go mempalace with ONNX, 33,746 LoC Go total. ONNX in `internal/embedder/hugot.go`, vector store `storage/govector/store.go`.

| Aspect | Assessment | Evidence |
|---|---|---|
| Lines of code | 3,500-4,500 Go (claimed). Plausible lower bound: existing laputa 3,182 + ~200-500 to wrap mempalace-style BM25 + ONNX adapter. | `projects/laputa/` `wc -l` = 3,182; `projects/mempalace-go-redis-v2/` = 33,746 (much larger). |
| Risk | High — introduces **second runtime** parallel to Rust diva. Cross-language IPC or shelling required at `memory_boundary.rs` or new wrapper. The current LaputaService is 100% Rust (`service.rs` no `Command::new`); replacing it would mean a hard fork. | No existing Go↔Rust bridge in `agent-diva/`. LAPUTA.md `§0` and `§4` establish "Laputa = subject-file substrate, agent owns". |
| Blast radius | `agent-diva-agent/src/memory_boundary.rs:16-35` (provider selection), `agent-diva-laputa/` (entire crate possibly replaced), `Cargo.toml` (drop `agent-diva-laputa` dep). 14 JSON sections at `.laputa/sections/*` must stay byte-compatible (LAPUTA.md `§1`). |
| Solves user's problem? | Partial. Unifies governance↔memory **only inside the new Go binary**. Does **not** extend the Rust `HybridMemoryProvider`'s typed-method integration (α's goal) unless the Rust side also learns a new client. Two problems conflated. |
| Reversibility | Hard. Once `.laputa/sections/*.json` schemas diverge, reverting requires data migration. |
| Discoverability | Low for new contributor: dual runtime, Go schema not visible from Cargo tree. |
| Compile-time impact | Removes `agent-diva-laputa` from workspace. New build target (Go). No Cargo feature flag. |
| First commit | "Drop in Go binary, wire `Command::new` shim in `LaputaService::open`" — first commit alone is a working regression. |
| Constraints | **Contradicts user**: user stated "Go version is not the final target" (per context). Also contradicts `AGENTS.md` "common workspace conventions" and LAPUTA.md `§4.4` "Laputa is file-first, Rust-native" implicitly (no Go crate in workspace). |

---

## Candidate B3 — Go deep-fusion (already disfavored)

> Rewrite `SectionStore` in Go to merge 14 section governance with mempalace's
> palace graph as a single schema. JSON for section metadata, palace graph for
> entries. ~6,000-8,000 lines Go.

| Aspect | Assessment | Evidence |
|---|---|---|
| Lines of code | 6,000-8,000 Go. Plausible: ~3,200 existing laputa + ~1,500-3,000 from mempalace-go's relevant subset (excluding web/redis). | `projects/laputa/` 3,182 LoC. Mempalace-go is 33,746 LoC; trimming to core store/graph could yield 6,000-10,000. |
| Risk | **Very high**. Merges two governance layers (proposal flow + diary entries) into one Go schema. The Rust LaputaService currently has 6 governance invariants (LAPUTA.md `§3`) including flock, atomic write, 30-day rollback, audit. These would all need to be re-implemented and proven in Go. | LAPUTA.md `§3` lists 6 mandatory write-phase guards. `agent-diva-laputa/src/service.rs:300-401` `rollback_changelog` implements 30-day invariant. |
| Blast radius | Entire `agent-diva-laputa/` crate (or its deprecation), `memory_boundary.rs`, `.laputa/` directory schema, every consumer (`agent-diva-autodream`, `agent-diva-manager`). Migration story undefined. |
| Solves user's problem? | **Conflates two distinct problems** (governance schema merge + memtle integration). Even if shipped, doesn't replace α's `HybridMemoryProvider` typed-method extension. |
| Reversibility | Essentially irreversible. The merged schema is the data model. |
| Discoverability | Very low. Requires reading Go + learning Rust tool definitions both. |
| Compile-time impact | Removes Rust laputa; introduces Go build. No Cargo feature flag. |
| First commit | Too coarse for atomic discipline (MOREDIVA/AGENTS.md "Atomic Commit Discipline"). A merge rewrite cannot be split into single-concern commits cleanly. |
| Constraints | **Same Go-vs-Rust objection as B2**, plus the schema-merge conflates independent concerns (per LAPUTA.md §10 "3 key boundaries"). |

---

## Candidate C3 — laputa + built-in lite backend (already disfavored)

> Keep `laputa.exe`. Add `internal/store/sqlite/sqlite.go` (~200 LoC) with
> FTS5. No ONNX. ~3,500-4,000 lines Go. User's stated objection: "性能妥协".

| Aspect | Assessment | Evidence |
|---|---|---|
| Lines of code | 200 LoC SQLite store + 3,182 existing = ~3,400. Plausible. | `projects/laputa/internal/store/redis/store.go` is a reference for backend style; `internal/` has 5 subpackages (rhythm/scheduler/store/wakeup/web). |
| Risk | Moderate. **Performance claim is non-trivial to substantiate** — FTS5 vs BM25-via-mempalace is a benchmark question. The Go binary does not currently have FTS5 (`grep -i sqlite\|fts5` returns 0 in `projects/laputa/`). | `projects/laputa/internal/store/` contains only `redis/`. |
| Blast radius | `projects/laputa/internal/store/sqlite/` new file; `agent-diva/` unchanged (still shells out — but per fact-check **it does not shell out today**, so this candidate implies adding a shell-out, contradicting LAPUTA.md). |
| Solves user's problem? | No. Solves search latency inside the Hermes plug-in path. Does **not** unify governance↔memory in Rust diva and does **not** extend `HybridMemoryProvider`. |
| Reversibility | Easy (additive file). |
| Discoverability | Moderate. |
| Compile-time impact | None on Cargo. New `modernc.org/sqlite` or `mattn/go-sqlite3` CGO dep. CGO breaks Windows cross-compile in many Go setups. |
| First commit | Add SQLite backend file + build tag. Tiny. |
| Constraints | Out of diva's scope — Hermes plug-in path. LAPUTA.md `§0` states Laputa is Rust-native; this reintroduces Go. Even within Hermes path, the "性能妥协" claim has no benchmark attached. |

---

## Candidate α — Extend HybridMemoryProvider in place

> Modify `agent-diva-core/src/memory/hybrid.rs` to invoke all 22 `MemtleToolkit`
> methods instead of 3 typed + 1 JSON-dispatched. ~150-350 lines of additions.

| Aspect | Assessment | Evidence |
|---|---|---|
| Lines of code | 150-350 LoC. Reference: each new method wrap is ~5-15 LoC; 18 new typed wrappers ≈ 90-270 LoC. Plus 2-4 small dispatch helpers in `prefetch` / `sync_turn` to call new methods. | `hybrid.rs:447-516` shows existing pattern: `call_json("memtle_diary_write", history_diary_write_args(history_entry))` is 4 LoC; the typed equivalent `toolkit.diary_write(DiaryWriteArgs{...})` is similar size. |
| Risk | Low. **No architectural changes.** Pure extension. No new crate. No new trait. Existing test scaffold (`hybrid.rs:552-558` `open_toolkit`, `:682` `add_drawer` typed call) shows the testing pattern already works. | `hybrid.rs:1-4` docstring; `hybrid.rs:534-961` 427 lines of tests. |
| Blast radius | `agent-diva-core/src/memory/hybrid.rs` only. Tests in same file. No Cargo.toml change. | Only `memtle` dep already present in `agent-diva-core/Cargo.toml:56-58`. |
| Solves user's problem? | **Half.** Closes the "memtle integration: 3 typed + 1 JSON → 22 typed" gap perfectly. **Does not** unify governance↔memory (no LaputaService dependency). The user's stated goal is "unify governance ↔ memory interface + extend memtle integration" — α addresses only the second half. |
| Reversibility | Very easy. Revert one PR. |
| Discoverability | High. Anyone reading `hybrid.rs` sees the full Memtle surface. |
| Compile-time impact | None. `mentle` feature already optional. |
| First commit | Replace the JSON-dispatched `memtle_diary_write` call with the typed `toolkit.diary_write(DiaryWriteArgs{...})` (`hybrid.rs:447`). Smallest possible slice. ~5 LoC. |
| Constraints | **Architectural ceiling**: cannot reach LaputaService. `agent-diva-core/Cargo.toml:12-58` has no path to `agent-diva-laputa` (and per `agent-diva-laputa/Cargo.toml:12` the direction is wrong). Governance↔memory unification blocked at the Cargo level. |

---

## Candidate β — New `agent-diva-garden` crate as adapter/wrapper

> New Rust crate depending on `agent-diva-laputa` (governance) AND `memtle`
> (memory). Exposes `recall_for_wakeup`, `archive_session`,
> `propose_milestone`, `sync_governance_to_memory`, `sync_memory_to_governance`.
> Implements `MemoryProvider`. ~500-800 lines.

| Aspect | Assessment | Evidence |
|---|---|---|
| Lines of code | 500-800 LoC. Plausible: trait impl (~120), 5 business methods (~30 each = 150), governance bridge (~80), memory bridge (~80), tests (~150). | Reference: `LaputaMemoryProvider` is 331 LoC incl. 4 tests (`memory_provider.rs:1-331`); `HybridMemoryProvider` is 961 LoC incl. 13 tests. Garden would combine patterns. |
| Risk | Moderate. New crate adds workspace member, two new edges in `Cargo.toml`, Cargo feature flag coordination. The "business method" names (`recall_for_wakeup`, etc.) are **not currently defined** anywhere — semantics are TBD. | `agent-diva/Cargo.toml:3-19` workspace members; adding a member is a one-line change but triggers `just ci` for all crates. |
| Blast radius | New `agent-diva-garden/` directory. Workspace root `Cargo.toml` adds member. New dep edges: `garden → laputa`, `garden → memtle`, `garden → core`. `agent-diva-agent/Cargo.toml:19` adds `agent-diva-garden = { path = "../agent-diva-garden" }`. `memory_boundary.rs:16-35` and `mentle_runtime.rs:73-75` may need updating to select the new provider. |
| Solves user's problem? | **Yes, both halves.** Garden has access to both laputa (governance) and memtle (memory). Implements `MemoryProvider` so it drops into the existing `Arc<dyn MemoryProvider>` slot at `agent_loop.rs:143`. |
| Reversibility | Moderate. Delete crate, remove from `Cargo.toml`, revert 1-2 callers. Leaves no orphan data. |
| Discoverability | High. A new contributor can `git grep recall_for_wakeup` and find the only place. |
| Compile-time impact | `agent-diva-garden/Cargo.toml` adds `agent-diva-core` (existing), `agent-diva-laputa` (path), `memtle` (workspace). Optional feature `mentle` to gate `memtle` dep, mirroring `agent-diva-core/Cargo.toml:60-62`. |
| First commit | Create empty crate + add to workspace. Add `Garden::new(workspace)` constructor. Wire it as `Arc<dyn MemoryProvider>` at `memory_boundary.rs:16`. (No business logic yet — proves the wiring.) |
| Constraints | Respects all `AGENTS.md` conventions: small modules, explicit types, documented APIs. Cargo dep direction is fine: garden sits **above** both laputa and core. The "Mentle integration is frozen to the published crates.io package at 0.1.2" rule (`agent-diva/Cargo.toml:87-89`) is satisfied by using workspace dep. |

---

## Candidate γ — New `agent-diva-garden` crate as abstract backend layer

> Like β, but adds `trait MemoryBackend` that `MentleBackend` and
> `LaputaBackend` implement. Garden holds `Vec<Box<dyn MemoryBackend>>`.
> ~800-1100 lines.

| Aspect | Assessment | Evidence |
|---|---|---|
| Lines of code | 800-1100 LoC. Extra ~300-400 over β for: `MemoryBackend` trait definition (~80), two concrete impls (~100 each), registry/fan-out dispatch logic (~80-150). | Existing pattern reference: `Tool` trait at `agent-diva-tooling` (not read, but `mentle_runtime.rs:279` `impl Tool for MentleToolkitTool` shows the trait-object dispatch style). |
| Risk | Moderate-high. **Adds a trait without a second concrete backend.** YAGNI risk: "future-proof for swappable backends" implies a second backend that doesn't exist yet. The two backends today are laputa (governance, already has a `MemoryProvider` impl in `LaputaMemoryProvider`) and memtle (memory toolkit). Wrapping both behind a new trait is a refactor, not new functionality. | `LaputaMemoryProvider:90`, `HybridMemoryProvider:334`, `MemoryManager` (in `manager.rs:15`). Three providers today, all behind `MemoryProvider`. A fourth abstraction layer (`MemoryBackend`) is novel. |
| Blast radius | All of β's, plus the trait definition impacts how garden talks to laputa and memtle internally. LaputaMemoryProvider does **not** implement `MemoryBackend` (it's a `MemoryProvider`). So LaputaBackend is a new wrapper that adapts `LaputaService` to `MemoryBackend`, not to `MemoryProvider`. |
| Solves user's problem? | Same as β for the user's stated goal. The "future-proof design" is speculative; no second backend exists or is planned. |
| Reversibility | Harder than β. Removing the trait requires unwrapping all call sites. |
| Discoverability | Mixed. Trait is documented, but the dispatch loop is non-obvious. |
| Compile-time impact | Same as β. |
| First commit | Same as β. Trait definition is a separate, small second commit. |
| Constraints | Respects workspace conventions. Slight conceptual overhead — MOREDIVA/AGENTS.md `§4 TRUTH_IN_CODE` requires the second backend to actually exist in code before being designed for. |

---

## Candidate X4 — Trait injection to bypass Cargo dependency direction

> Keep `hybrid.rs` as implementation site. Define `trait GovernanceAuthority`
> in `agent-diva-core`. `agent-diva-laputa` provides blanket impl for
> `LaputaService`. Hybrid receives `Arc<dyn GovernanceAuthority>` via
> constructor. ~200-400 lines. No new crate.

| Aspect | Assessment | Evidence |
|---|---|---|
| Lines of code | 200-400 LoC. Plausible: trait definition in `agent-diva-core` (~40), blanket impl in `agent-diva-laputa` (~40), constructor wiring in `hybrid.rs` (~50), 2-3 governance-aware methods (~100), tests (~80). | Reference: `MemoryProvider` trait is ~30 LoC (`provider.rs:384-415`); a thinner `GovernanceAuthority` would be similar. |
| Risk | Moderate. **Inverts a property of LAPUTA.md's authority boundary**: the laputa crate currently has a *read-only* `LaputaMemoryProvider` (per `memory_provider.rs:1` docstring "Read-only Laputa adapter"). Defining `GovernanceAuthority` as a trait in `agent-diva-core` (the lower crate) and having `agent-diva-laputa` (higher crate) implement it would mean **laputa depends on a trait defined downstream**, which is structurally fine in Rust but introduces a new "downstream trait, upstream impl" pattern. Acceptable but unusual. | `provider.rs:384` is the analogous lower-crate-defined trait. `agent-diva-laputa/Cargo.toml:12` already imports `agent-diva-core`, so the new trait is visible to laputa. |
| Blast radius | `agent-diva-core/src/memory/governance.rs` (new, defines trait). `agent-diva-laputa/src/governance_authority_impl.rs` (new, blanket impl). `agent-diva-core/src/memory/hybrid.rs` modified to take `Arc<dyn GovernanceAuthority>` in `new()`. `mentle_runtime.rs:74` updated to construct and pass the impl. No workspace Cargo.toml change. |
| Solves user's problem? | **Yes, both halves.** Hybrid gets governance access; memtle integration extends in place (α's scope also included). |
| Reversibility | Easy. Remove trait + impl, revert constructor signature. |
| Discoverability | High for trait users; low for new contributors who may not know to look in `agent-diva-core/memory/` for a trait that laputa implements. |
| Compile-time impact | Zero new deps. Zero new workspace members. |
| First commit | Define empty `trait GovernanceAuthority` in `agent-diva-core`. Commit `cargo check`. |
| Constraints | Respects Cargo direction (laputa → core). Respects AGENTS.md "small modules" (trait lives in `agent-diva-core/src/memory/governance.rs`). Mentle package policy unaffected. |

---

## Candidate X5 — Embed the garden inside `agent-diva-laputa`

> No new crate. Add memory methods to `LaputaService`:
> `propose_milestone`, `archive_session`, `sync_with_memtle`. LaputaService
> gets `memtle` as optional dep. ~400-600 lines added to `service.rs`.

| Aspect | Assessment | Evidence |
|---|---|---|
| Lines of code | 400-600 LoC added to `service.rs`. Currently 810 LoC; would grow to ~1,200-1,400. | `service.rs` `wc -l` = 810. Reference: each existing `pub fn` (read_snapshot, rollback_changelog, etc.) is 20-100 LoC. |
| Risk | High. **Violates LAPUTA.md §0 boundary**: "Laputa = subject-file substrate, NOT DB / Vector store". Adding `memtle` to laputa makes laputa depend on a backend storage engine. The `laputa` crate would no longer be "file-first" per LAPUTA.md `§4.4`. Also forces `memtle` into laputa's dep tree, which contradicts "Laputa 写、Mentle 读 (authority 单向)" (LAPUTA.md `§5`). | LAPUTA.md `§0`, `§4.4`, `§5` set the boundary. `agent-diva-laputa/Cargo.toml:11-19` currently has 7 deps, no memtle. |
| Blast radius | `agent-diva-laputa/Cargo.toml` adds optional `memtle` dep (must be gated — but laputa isn't feature-gated today). `agent-diva-laputa/src/service.rs` grows by ~500 LoC. Every consumer of laputa (`agent-diva-agent`, `agent-diva-autodream`, `agent-diva-manager`) now transitively pulls memtle unless laputa features gate it. |
| Solves user's problem? | Yes — laputa can call memtle directly. But solves it by **inverting the authority direction** (LAPUTA.md `§5` says laputa writes, memtle reads; X5 makes laputa call memtle). |
| Reversibility | Hard. The methods on `LaputaService` are part of the public surface (`service.rs:31` "Stable Laputa service API used by Rust callers, manager routes, and Tauri commands"). Removing them is a breaking change. |
| Discoverability | Confusing — laputa now has memory methods, blurring the line. |
| Compile-time impact | `agent-diva-laputa/Cargo.toml` adds `memtle = { workspace = true, optional = true }` + a feature flag. Force-propagates to all laputa consumers. |
| First commit | Add `LaputaService::memtle_status` that wraps `MemtleToolkit::open` + `status`. ~15 LoC. |
| Constraints | **Conflicts with LAPUTA.md design contract**: "`Laputa = subject-file substrate, NOT DB / Vector store / Graph DB`" and "`Laputa 写、Mentle 读 (authority 单向)`". Even if updated docs allow it, this is a design reversal. |

---

## Candidate X6 — Garden as a sync helper, not a provider

> New `agent-diva-garden` crate that does **not** implement `MemoryProvider`.
> Free functions / `Garden` struct that `HybridMemoryProvider` and
> `LaputaService` each call into. ~200-350 lines. Smallest blast radius.

| Aspect | Assessment | Evidence |
|---|---|---|
| Lines of code | 200-350 LoC. Plausible: 5 sync helpers (~30 each = 150), a `Garden::new()` + `Garden::sync_*` orchestration (~80), tests (~70). | Reference: `history_diary_write_args` in `hybrid.rs:509-516` is 8 LoC; `diary_write_succeeded` is 15 LoC. Each helper is similar. |
| Risk | Low. **Does not change provider selection** at `memory_boundary.rs:16-35` or `mentle_runtime.rs:73-75`. Both existing providers keep their slots; they just call into the same shared helper module when they need cross-store coordination. | `memory_boundary.rs:16` unchanged; `mentle_runtime.rs:73-75` unchanged. |
| Blast radius | New `agent-diva-garden/` crate + workspace member. Two new dep edges (laputa→garden, memtle→garden, or both→garden). `HybridMemoryProvider::sync_turn` and `LaputaService::apply_proposal` each get a one-line call into garden for cross-store coordination. No provider-selection change. |
| Solves user's problem? | **Partial.** Solves cross-store sync (governance↔memory) cleanly. Does **not** unify the `Arc<dyn MemoryProvider>` slot — laputa and memtle remain disjoint providers. This means `agent_loop.rs:490-492` "mentle overrides only if laputa isn't chosen" stays the same. |
| Reversibility | Easy. Delete crate. The call sites are inline function calls; reverting is a 1-2 line patch per site. |
| Discoverability | High — the helpers are imported and used inline; readers can `git grep` for each helper. |
| Compile-time impact | New workspace member + 2-3 dep edges. |
| First commit | Create crate with one helper: `garden::sync_history_to_diary(history_entry: &str, toolkit: &MemtleToolkit) -> Result<()>`. Have `HybridMemoryProvider::sync_turn` call it instead of inline `call_json`. ~30 LoC including test. |
| Constraints | Respects all workspace conventions. Does not violate LAPUTA.md (laputa and mentle remain separate; garden is just a sync glue layer). Smallest behavioral surface of all garden variants. |

---

## Comparison matrix (9×9, with totals)

Scoring criteria (1=poor, 5=excellent). All scores are evidence-based.

### Criteria

1. **Solves user's stated problem** (governance↔memory unify + memtle extend)
2. **Reversibility** (ease of undo)
3. **Discoverability** (obviousness to a new contributor)
4. **Compile-time hygiene** (no new deps / new workspace members / new feature flags where avoidable)
5. **Conformity to existing constraints** (LAPUTA.md authority boundary, Cargo direction, memtle version pin, AGENTS.md)
6. **Atomic discipline** (can the work be split into single-concern commits per MOREDIVA/AGENTS.md §Atomic Commit Discipline?)
7. **Test-friendliness** (existing test infra reusable?)
8. **Reversibility-of-data** (does it mutate `.laputa/` schema or `palace.db` schema?)

### Matrix

| Candidate | Solves problem | Reversibility | Discoverability | Compile hygiene | Constraint fit | Atomic discipline | Test-friendliness | Data reversibility | **Total** |
|---|---|---|---|---|---|---|---|---|---|
| **B2** Go mid-fusion | 2 (partial) | 1 (hard) | 1 | 1 (drops Rust crate, adds Go) | 1 (contradicts "Go not final") | 1 (cannot split) | 1 | 1 | **9** |
| **B3** Go deep-fusion | 2 (partial) | 1 | 1 | 1 | 1 | 1 | 1 | 1 | **9** |
| **C3** laputa+sqlite | 1 (wrong scope) | 4 (additive file) | 3 | 2 (no Cargo impact) | 2 (out of scope) | 4 | 3 | 4 | **23** |
| **α** extend hybrid | 2 (half: memtle only) | 5 | 5 | 5 | 5 | 5 | 5 | 5 | **37** |
| **β** garden adapter | 5 (both halves) | 4 | 5 | 3 (new crate + 2 deps) | 5 | 5 | 4 | 4 | **35** |
| **γ** garden abstract | 5 | 3 | 3 (extra layer) | 3 | 4 (YAGNI) | 4 | 3 | 4 | **29** |
| **X4** trait injection | 5 | 4 | 4 | 5 (zero new deps) | 5 | 4 | 4 | 5 | **36** |
| **X5** laputa+memtle | 5 (inverted direction) | 2 (breaks public API) | 2 (confusing) | 2 (propagates memtle) | 1 (violates LAPUTA.md §5) | 2 | 3 | 2 | **19** |
| **X6** garden helper | 3 (half: sync only, no provider unify) | 5 | 5 | 3 | 5 | 5 | 4 | 5 | **35** |

### Per-row justification (one sentence each)

- **B2 (9)**: Go binary contradicts user's stated preference; cross-language boundary breaks Rust-only test suite; data migration for `.laputa/` format not addressed.
- **B3 (9)**: Same Go objection as B2 plus deep schema merge conflates two governance layers from LAPUTA.md §3; effectively irreversible.
- **C3 (23)**: Out of diva's scope (Hermes plug-in path); user objection "性能妥协" unverified by benchmark; but cheap and additive.
- **α (37)**: Smallest delta, zero new deps, perfectly fits existing test scaffold (`hybrid.rs:534-961`); ceiling is the Cargo direction — cannot reach laputa.
- **β (35)**: Solves both halves of the user's goal, fits workspace conventions; new crate adds 2 dep edges and a workspace member (minor compile-time cost).
- **γ (29)**: Same as β plus a speculative `MemoryBackend` trait with only one planned second impl — YAGNI overhead without observed need.
- **X4 (36)**: Solves both halves with zero new deps and no new crate; "downstream trait, upstream impl" is structurally unusual but valid in Rust.
- **X5 (19)**: Solves the problem by inverting LAPUTA.md §5's authority direction (laputa writes, memtle reads); laputa grows by ~50% with new public surface; forces memtle into laputa's dep tree.
- **X6 (35)**: Smallest blast radius of garden variants, but does not unify the `Arc<dyn MemoryProvider>` slot — leaves `memory_boundary.rs:16-35` disjoint.

### Ranking by total (descending)

1. **α — 37**
2. **X4 — 36**
3. **β — 35** (tie)
3. **X6 — 35** (tie)
5. **γ — 29**
6. **C3 — 23**
7. **X5 — 19**
8. **B2 — 9** (tie)
8. **B3 — 9** (tie)

(Note: this is a score ranking, **not** a recommendation. The user has stated B2/B3 are disfavored; α is also constrained by Cargo direction; X4 and β address both halves of the goal. The user must choose.)

---

## Compatibility with user constraints (B2/B3/C3 vs stated Go-vs-Rust preference)

**User's stated preference**: "Go version is not the final target" (per context).

| Candidate | Language | Compatibility | Notes |
|---|---|---|---|
| B2 | Go (new binary) | ❌ Contradicts user. Reintroduces Go in a codebase that is 100% Rust (`agent-diva/` workspace members = 15 Rust crates, 0 Go crates). |
| B3 | Go (rewrite) | ❌ Contradicts user, more invasive than B2 (replaces Rust laputa entirely). |
| C3 | Go (additive) | ⚠️ Out of diva's scope. The external Go `laputa.exe` at `projects/laputa/` is documented in LAPUTA.md as the Hermes plug-in path, not diva's runtime. Even within that path, no `sqlite/fts5` backend exists today (`grep fts5\|sqlite` in `projects/laputa/` returns 0 matches). |

**Rust-only candidates** (α, β, γ, X4, X5, X6): all consistent with the user's preference. X5 adds an optional `memtle` dep to laputa; the others either consume `memtle = 0.1.2` at existing dep sites (α, β, γ, X6) or add zero new deps (X4).

**Dep-direction note**: LAPUTA.md `§5` "Laputa 写、Mentle 读 (authority 单向)" means LaputaService must NOT call into MemtleToolkit in production. X5 violates this directly. β/γ/X6 work around it by routing through a new crate; α accepts the gap and doesn't try to cross it; X4 inverts via a downstream trait that laputa implements, which preserves the authority direction (laputa still writes; it just *exposes* a trait surface that hybrid consumes).

---

## Summary of evidence sources

- `agent-diva/Cargo.toml:1-19` (workspace members, memtle pin at :89)
- `agent-diva-core/Cargo.toml:12-67` (memtle feature gate, no laputa dep)
- `agent-diva-core/src/memory/provider.rs:384-415` (trait)
- `agent-diva-core/src/memory/hybrid.rs:1-4, 286-507, 534-961` (current implementation + tests)
- `agent-diva-core/src/memory/manager.rs:15-243` (default provider)
- `agent-diva-laputa/Cargo.toml:11-19` (depends on core, no memtle)
- `agent-diva-laputa/src/service.rs:31-810` (public API: `metrics_snapshot`, `open`, `create_proposal`, `get_proposal`, `list_proposals`, `edit_proposal`, `transition_proposal`, `apply_proposal`, `apply_proposal_with_options`, `create_and_apply_direct_edit`, `read_snapshot`, `read_section`, `list_changelog`, `get_changelog`, `rollback_changelog`, `poll_events`, `subscribe_events`, `replay_events` — 18 pub fn)
- `agent-diva-laputa/src/memory_provider.rs:1-331` (LaputaMemoryProvider, read-only)
- `agent-diva-laputa/src/layout.rs:87-164` (14 sections stored as JSON files)
- `agent-diva-agent/src/agent_loop.rs:143, 317, 444-505` (provider slot + mentle override logic)
- `agent-diva-agent/src/memory_boundary.rs:16-99` (provider selection + DegradedMemoryProvider)
- `agent-diva-agent/src/mentle_runtime.rs:73-75, 87-94` (mentle wiring)
- `.workspace/memtle/src/toolkit.rs:27-210` (22 MemtleToolkit methods confirmed)
- `projects/laputa/` 3,182 LoC Go, no SQLite/FTS5 today
- `projects/mempalace-go-redis-v2/` 33,746 LoC Go, includes ONNX + vector store
- `LAPUTA.md §0, §3, §4.4, §5, §10` (design contract: file-first, authority单向)
- `agent-diva/AGENTS.md` (workspace conventions, atomic commits, memtle freeze)

No facts in the user's context were contradicted. All scores above are derived from the cited evidence only.
