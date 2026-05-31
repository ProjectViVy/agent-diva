# Mentle, Laputa, and Rhythm Memory Role Decision

> Status: accepted architecture direction.
> Date: 2026-05-31.
> Scope: define Mentle's role after the GenericAgent + Laputa + prompt-governed rhythm evolution simplification.

## 1. Decision

Agent-Diva should not treat Mentle as always-injected long memory.

The new memory architecture should use this split:

- **GenericAgent**: runtime trunk, agent loop, tools, sessions, heartbeat, event bus, context assembly.
- **Laputa**: subject file layer and authority-bearing continuity files.
- **AutoDream / Rhythm Evolution**: prompt-governed reflection over GenericAgent evidence and Laputa files.
- **Mentle**: optional external semantic memory tool, recall/index layer, and temporary work-memory store.
- **Harness / prompt policy / review**: decides when memory should be searched, written, proposed, approved, or rejected.

Mentle content should **not** enter the normal prompt context by default. It should behave like an external notebook that Diva can open when needed, not like permanent material already inside Diva's active mind.

## 2. Core Model

Use this mental model:

```text
Laputa = authority-bearing subject file layer
MEMORY.md = high-density long-term subject memory entry point
Mentle = external semantic notebook and tool-call memory
Context = current working mind / prompt workspace
```

Or:

```text
GenericAgent runtime
  -> Laputa subject files
  -> LLM rhythm prompt
  -> Evolution proposals
  -> Review / policy
  -> Authority file updates

Mentle
  <-> optional recall / write / update / delete tool calls
  <-> optional index over Laputa, reports, sessions, and project notes
```

## 3. Mentle Is Not Default Context

Mentle should not be automatically injected into daily conversation context.

Reasons:

- It can add unrelated or stale material to the prompt.
- It can make Diva over-trust old notes.
- It can blur the boundary between current conversation and historical recall.
- It can increase context size without a clear current-task benefit.

Daily use should treat Mentle as an optional tool:

```text
user conversation
  -> harness / prompt decides whether recall is needed
  -> optional Mentle search/read
  -> selected results enter current context only for this task
```

This keeps Mentle similar to a notebook on the desk: available to open, but not already in the active context.

## 4. Mentle Wings Simplification

Mentle should expose a small set of high-level wings rather than many specialized tools.

Preferred first shape:

```text
mentle.search(query, filters)
mentle.read(id)
mentle.write(note)
mentle.update(id, patch)
mentle.delete_or_archive(id)
```

Optional later shape:

```text
mentle.link(source_id, target_id, relation)
```

The design goal is to reduce a large tool surface into understandable CRUD-style operations.

## 5. What Should Be Written to Mentle

Mentle is appropriate for external notes, project knowledge, semantic recall, and temporary work memory.

High-priority write candidates:

- user explicitly says to remember or record something;
- project research summaries;
- technical conclusions;
- reusable procedures;
- command notes and troubleshooting records;
- project decisions, constraints, and open questions;
- daily, weekly, and monthly report indexes;
- AAAK summaries of rhythm reports;
- AutoDream intermediate material that is not yet authority memory;
- major-task temporary work memory;
- source references that may help future recall.

Lower-priority or should-not-write candidates:

- one-off casual chat;
- unstable emotional fragments;
- inferred personality judgments without user confirmation;
- sensitive private information without explicit authorization;
- raw model speculation about the user;
- facts already reliably available in project files;
- content that belongs in Laputa authority files instead;
- anything that would silently become long-term identity memory without review.

The key rule:

> Mentle can store external notes and work material. Laputa / MEMORY.md stores subject continuity and authority-bearing long-term memory.

## 6. What Should Enter Laputa Instead

Laputa should own Diva's subject file layer.

Examples:

- identity files;
- relationship and commitment files;
- daily, weekly, and monthly reports;
- journal/reflection artifacts;
- evolution proposal inbox;
- `MEMORY.md`;
- `HISTORY.md`;
- memory or subject-file changelog;
- reviewable long-term memory patches.

GenericAgent should not directly scatter these files across runtime modules. It should call a Laputa-facing boundary or store abstraction.

## 7. AutoDream and Mentle

AutoDream can integrate Mentle later, but not as the authority source.

Future flow:

```text
AutoDream / Rhythm Evolution
  -> read recent sessions
  -> read Laputa files
  -> optionally call Mentle recall for related history, project notes, and report indexes
  -> generate daily / weekly / monthly reports
  -> generate EvolutionProposal records
  -> write proposals to Laputa inbox
  -> optionally write searchable intermediate notes to Mentle
  -> update MEMORY.md or identity files only after review / policy approval
```

Mentle can suggest and retrieve. It must not authorize durable changes.

## 8. Daily Conversation and Harness Policy

Daily conversation can write memory, but the core decision of what should be written should come from harness / prompt policy and review rules.

Expected behavior:

- default conversation does not call Mentle;
- call Mentle when the user asks for recall, the task clearly depends on prior work, or the harness detects a high-value recall need;
- write to Mentle when the user explicitly asks to record something or when the current work produces reusable external notes;
- do not turn casual conversation into long-term identity memory without confirmation;
- do not inject Mentle results unless the current task needs them.

This keeps daily use lightweight while still allowing high-value memory capture.

## 9. Rhythm Reports, Indexes, and AAAK Compression

Daily, weekly, and monthly reports should live in Laputa.

Each report can have a compact index entry and an AAAK-style compressed summary.

Recommended retrieval path:

```text
1. Search report indexes / AAAK summaries.
2. Select likely relevant daily / weekly / monthly reports.
3. Read the full report files.
4. If needed, read original sessions or source material.
```

This avoids injecting all reports into context and gives Diva a staged recall path.

Mentle can index:

- report titles;
- dates and periods;
- participants or projects;
- AAAK summaries;
- keywords;
- links to full Laputa report files;
- links to source sessions when available.

The report entity remains in Laputa. Mentle stores or indexes recall handles.

## 10. Mentle as Temporary Work Memory

Mentle can serve as temporary memory during major work.

Example structure:

```text
work_memory/<project_or_task>
  - current_goal
  - constraints
  - decisions
  - open_questions
  - evidence
  - next_actions
  - useful links
```

This material does not need to live in active prompt context all the time.

During a major task:

```text
task start
  -> recall work_memory
  -> load selected parts into context

task progress
  -> update work_memory when durable intermediate state appears

task end
  -> AutoDream / Laputa decides what becomes report, proposal, archive, or long-term memory
```

This gives Diva better long-work continuity without treating every temporary note as identity memory.

## 11. Obsidian Direction

Future Obsidian integration fits the same split:

```text
Laputa files
  -> human-readable authority and rhythm files

Obsidian
  -> human-facing linked note workspace / knowledge garden

Mentle
  -> machine-facing semantic index and recall engine
```

Mentle may index Obsidian and Laputa files, but the authority material should remain readable, editable, versionable files.

## 12. Authority and Write Safety

Mentle is not the source of truth for subject continuity.

Authority order:

1. Laputa authority files and `MEMORY.md`.
2. Laputa changelog and proposal review records.
3. Full rhythm reports and journal artifacts.
4. Mentle indexes, notes, and temporary work-memory entries.

Rules:

- Mentle may recall.
- Mentle may store external notes.
- Mentle may support AutoDream with related evidence.
- Mentle may keep temporary work memory.
- Mentle must not silently rewrite `MEMORY.md`.
- Mentle must not silently rewrite identity files.
- Mentle must not become default prompt context.

## 13. Implementation Order

Recommended order:

1. Keep P0 AutoDream / Rhythm Evolution file-first and prompt-governed.
2. Place identity files, daily/weekly/monthly reports, proposals, and changelogs in Laputa.
3. Define the simplified Mentle wings as an optional tool interface.
4. Add report indexes and AAAK summaries as Laputa artifacts that Mentle can index.
5. Add major-task work-memory support in Mentle as an optional external store.
6. Later connect AutoDream to Mentle recall for related history and report lookup.
7. Later connect Obsidian as a human-facing note surface.

## 14. Principle

Use this principle in future design reviews:

> Mentle is an external semantic notebook and recall tool. It can help Diva remember, but it should not be Diva's always-active mind or the authority source for subject continuity.

