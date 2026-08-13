# Laputa Memory Architecture Decision

> Repatriated from `refactor/deep-governance` on 2026-07-30 and accepted as the
> target architecture for current `agent-diva-pro`. Statements that code “has
> already been deleted” describe the source branch, not the current branch.
> Current implementation status and ordering are tracked by GMH-23A..24 in the
> root `TODOLIST.md`.

- Status: **Accepted**
- Date: 2026-07-24
- Decision scope: Laputa-Garden, Laputa-Diva, embedded long-term memory, synchronization, Mentle clean break
- Implementation status: **GMH-23A contract frozen; GMH-23B implementation pending**

## GMH-23A frozen implementation contract

The following choices are final for the current `agent-diva-pro` implementation:

- The canonical local database is `<workspace>/.laputa/memory.sqlite3`.
  Workspace placement is the physical profile boundary; every record also
  carries mandatory tenant/workspace and optional session scope.
- `agent_diva_core::memory::MemoryRecord` is the only canonical typed record.
  SQLite columns are indexes over that contract, never a competing domain model.
- The store uses the workspace `sqlx` SQLite runtime, schema versioning, WAL,
  transactions, expected-revision CAS, and tombstones. It must not silently
  overwrite or physically delete authority records.
- FTS5 with `unicode61` and BM25 produces bounded candidates only. Retrieval
  cannot grant trust or authority and is not connected to Recall v2 until
  GMH-23C.
- Existing file-first proposals, sections, changelog, audit, rollback, and
  Manager/Tauri APIs remain unchanged during GMH-23B. There is no dual write or
  production cutover.
- Legacy Markdown and Laputa JSON are eligible only for a separately invoked,
  reversible offline import at GMH-24. Old Mentle databases are never read.
- Garden, synchronization, persona capsule, GUI product work, and remote
  MemoryOS protocols are outside GMH-23A/23B.

### Data ownership and flow

```text
evidence / user edit
        |
        v
file-first pending proposal ----X----> typed authority
        |                    no GMH-23B apply path
        v
future governed apply (GMH-23D)
        |
        v
MemoryRecord transaction -> .laputa/memory.sqlite3 -> FTS candidate index
                                                        |
                                                        v
                                           Recall v2 adapter (GMH-23C)
```

Only a future governed apply path may mutate typed authority. FTS, AgentLoop,
Manager, Tauri, GUI, AutoDream, and imported evidence are not owners.

## Decision

Laputa is finalized as one memory architecture with two deliberately different product forms:

1. **Laputa-Garden** is the independent, full MemoryOS. It is the system-wide personality and long-term-memory authority for multiple agents, complete retrieval, progressive compression, governance, and Agentic RAG.
2. **Laputa-Diva** is the embedded lightweight form. It preserves Diva's install-and-run, single-process, offline-capable product philosophy while providing usable long-term memory and a durable local core-personality replica.

Laputa-Diva does not embed Garden and does not embed a reduced MemoryOS. It reuses Laputa's proposal, evidence, revision, decision, projection, and governance concepts over a new lightweight storage substrate inspired by GenericAgent's layered-memory discipline.

Mentle/MenPalace is removed from the Diva architecture. There will be no Mentle runtime dependency, database reader, compatibility adapter, feature flag, dual write, legacy schema fallback, or online migration path. Git history and historical research remain the archive. Any user-data migration that is later judged necessary must be an explicit offline import into the new Laputa model.

## Why this decision exists

The pro architecture never reached a satisfactory memory closure. Mentle was too heavy for Diva's embedded role, while removing long-term memory from Diva would violate its plug-and-play and offline-first value.

Garden changes the system boundary: it is now a MemoryOS that can serve multiple agents. Diva still needs an embedded memory engine, but that engine and Garden solve different problems.

The final boundary is therefore:

| Concern | Laputa-Diva | Laputa-Garden |
|---|---|---|
| Product role | Embedded lightweight memory | Independent MemoryOS |
| Availability | Default local, offline-capable | Optional external service |
| Scope | One Diva profile and its core working set | System-wide, multi-agent memory |
| Storage | Lightweight profile-local substrate | Garden-owned complete storage and indexes |
| Retrieval | Local core and degraded/offline recall | Complete retrieval and Agentic RAG |
| Personality | Durable local core replica | Full personality-memory authority |
| Raw/deep evidence | Selective only | Complete governed corpus |
| Synchronization | Bidirectional selective replica | Hub/authority side |

## Architectural model

```text
AgentLoop / ContextProvider
          |
          v
Laputa-Diva governance
  - proposal and evidence
  - exact revision decisions
  - local core personality
  - lightweight retrieval
  - sync journal/outbox
          |
          +---- profile-local lightweight store
          |
          +<== selective bidirectional sync ==> Laputa-Garden MemoryOS
                                                  |
                                                  +-- complete retrieval
                                                  +-- progressive compression
                                                  +-- multi-agent aggregation
                                                  +-- conflict governance
                                                  +-- Agentic RAG
```

## Embedded storage direction

GenericAgent is a design reference, not a source-code transplant or runtime dependency. Laputa-Diva should adopt these principles:

- a very small always-available top layer;
- minimum sufficient pointers to deeper memory;
- deeper facts, experiences, and SOPs loaded on demand;
- working memory separated from long-term memory;
- only evidence-backed information promoted into durable memory;
- progressive compression that keeps the most valuable source lineage.

The implementation must be rewritten under Diva's clean-break constraints:

- typed records and bounded values;
- profile-local isolation;
- CAS/revision semantics;
- explicit provenance and evidence digests;
- tombstones instead of silent destructive deletion;
- writes through the unique `ToolExecutionGateway`;
- Receipt/Audit for effectful changes;
- fail-closed policy and authorization;
- no model-directed arbitrary writes to authority files.

The canonical online store must not be a loose Markdown directory. A human-readable Memory Pack may be designed as an explicit import/export projection, not a second live authority.

GMH-23A closes the local-store questions left open by the original ADR:
SQLite/FTS5 is owned by `agent-diva-laputa`, uses the workspace `sqlx` runtime,
and stores canonical core records. Recall scoring beyond FTS5/BM25,
compression, and Garden synchronization remain intentionally undecided and
out of scope.

## Core personality replica

When Garden is configured, Diva treats Garden as the external complete memory database and uses Garden retrieval results in normal context resolution. Diva must nevertheless retain a bounded local core-personality working set so identity does not disappear when Garden is unavailable.

The local core must cover the minimum continuity set, including the accepted equivalents of:

- identity and durable boundaries;
- stable user preferences;
- important relationships;
- active commitments;
- durable goals;
- critical experience pointers;
- the revision and provenance necessary to explain each item.

This is a governed materialized view, not a static system prompt and not a full copy of all fourteen Laputa sections.

## Selective bidirectional synchronization

Synchronization is mandatory when Garden integration is enabled, but full replication is not.

### Upstream: Diva to Garden

Eligible committed changes include accepted facts, preferences, relationships, commitments, high-value experience summaries, revisions, retractions, and evidence-backed proposals. Working checkpoints, secrets, volatile state, unaccepted proposals, and raw sessions are excluded by default.

### Downstream: Garden to Diva

Garden sends accepted core-personality revisions, conflict decisions, promoted high-importance memories, important relationship or commitment changes, and bounded top-layer summaries required for offline continuity.

Garden query results are not automatically persisted locally. Search results enter the current Context package; only records selected by the replication policy enter the local core.

### Replication properties

The protocol must provide:

- stable source and memory identities;
- monotonic revisions or another researched causal model;
- idempotent batches and restart-safe cursors;
- transactional local outbox/inbox application;
- origin and lineage fields that prevent sync loops;
- tombstone propagation and no stale resurrection;
- explicit conflict state rather than last-write-wins for personality;
- per-section/record replication and privacy policy;
- honest offline, degraded, conflict, and backlog status.

The fourteen-section taxonomy remains a governance and projection concern. It must not imply that all sections are fully synchronized or that the physical store consists of fourteen authority files.

## Authority and failure behavior

- Garden is the full-memory authority for the connected MemoryOS corpus.
- Diva is the local authority for its offline commits until Garden accepts, rejects, or conflicts them.
- Core-personality conflicts require an explicit governed decision; timestamps alone cannot overwrite identity.
- A Garden outage never blocks base chat, local memory capture, or local core recall.
- A sync failure is durable backlog, not fake success.
- A local memory write and its outbound sync event must not be separable by a crash.
- Garden-derived material does not silently become local personality merely because retrieval returned it.

## Mentle clean break

The earlier `RG-E8-S4` Mentle integration direction and its Rust 1.88 dependency blocker are superseded by this decision.

Required removal research must inventory and then delete or replace:

- Mentle dependencies and feature flags;
- runtime/toolkit adapters;
- Manager routes and DTOs;
- GUI capability bindings and settings;
- tests, fixtures, and build checks that promise Mentle;
- stale architecture and product claims that present Mentle as a Diva backend.

No compatibility code may remain. Historical documents may remain only when clearly labeled superseded or archival. The current clean-break runtime must never read an old Mentle database.

## Required research before implementation

The research phase must produce evidence and an executable design for:

1. the current Diva Laputa contract and storage gap;
2. GenericAgent layered-memory principles and the parts safe to adapt;
3. Garden's actual API, revision, compression, identity, authentication, and failure contracts;
4. lightweight local-store candidates under Rust 1.80 and all supported platforms;
5. the local core-personality schema and bounded projection;
6. fourteen-section replication classification and privacy defaults;
7. bidirectional sync causality, conflict resolution, tombstones, loop prevention, and recovery;
8. online Garden retrieval plus offline/local result merging;
9. Memory Pack import/export and the offline-only legacy migration boundary;
10. complete Mentle deletion inventory and clean-break gates;
11. crate/API/Manager/GUI boundaries and unique-Gateway proof;
12. phased implementation, tests, rollback, and acceptance criteria.

Research may change mechanisms, schemas, and naming. It may not reverse the product decision without a new explicit ADR.

## Superseded directions

This decision supersedes:

- embedding Mentle/MenPalace in Diva;
- reopening Mentle solely by raising the workspace MSRV;
- making Diva a memoryless Garden client;
- choosing strictly between local memory and Garden;
- full fourteen-section database mirroring;
- runtime dual writes to SQLite and Markdown;
- compatibility readers for pro-era Mentle data.

## Acceptance boundary

This ADR and backlog update record a product and architecture decision only. They do not claim that:

- the lightweight engine exists;
- Garden synchronization exists;
- Mentle code has already been deleted;
- the Garden protocol is release-frozen;
- C-Parity is complete.

Those claims require the dedicated research and subsequent implementation slices.
