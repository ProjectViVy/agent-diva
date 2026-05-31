# Autonomous Evolution Simplified Architecture Decision

> Status: accepted working direction.
> Date: 2026-05-31.
> Scope: consolidate the latest decisions about GenericAgent, Laputa, AutoDream, rhythm evolution, Mentle, context compaction, and prompt-governed autonomous evolution.

## 1. Executive Decision

Agent-Diva autonomous evolution should be simplified from a heavy algorithmic memory architecture into a **GenericAgent-native, Laputa-file-first, prompt-governed rhythm evolution pipeline**.

The system should not start by building a complex autonomous memory operating system. The first useful version should be a small, reviewable loop:

```text
GenericAgent evidence
  -> Laputa subject files
  -> AutoDream prompt task
  -> rhythm report draft / evolution proposal
  -> review or policy
  -> authority file update with changelog
```

The current implementation direction should prioritize:

- clear naming;
- clean ownership boundaries;
- file-first subject continuity;
- prompt contracts instead of early complex scoring algorithms;
- reviewable memory proposals;
- minimal GenericAgent integration points;
- future extension hooks for Mentle, Obsidian, report indexing, and context compaction.

## 2. Core Split

Use this split as the current architecture baseline:

```text
GenericAgent
  runtime trunk: agent loop, sessions, tools, context builder, heartbeat, event bus, MemoryProvider boundary

Laputa
  subject file layer: identity files, relationship files, commitments, MEMORY.md, HISTORY.md,
  daily/weekly/monthly reports, proposal inbox, changelog

AutoDream
  background/manual reflection task that reads GenericAgent evidence and Laputa files,
  optionally calls Mentle in the future, and writes drafts/proposals back to Laputa

Rhythm Evolution
  heartbeat-based daily/weekly/monthly triggering and product shape around AutoDream

Mentle
  optional external semantic notebook, recall/index layer, CRUD-style tool surface,
  and temporary work-memory store

Context Compaction
  session-local prompt survival mechanism; separate from AutoDream and rhythm evolution
```

## 3. From Algorithm Engineering to Prompt Engineering

The current direction intentionally simplifies the earlier architecture.

Earlier direction:

```text
complex scoring / resonance / evidence graph / full audit chain / capsules / automatic governance
```

Current P0 direction:

```text
file protocol + prompt contract + user/policy review + changelog
```

This is not a weaker architecture. It moves uncertainty to the layer that can be iterated fastest.

Early work should avoid:

- complex resonance computation;
- graph-based personality inference;
- automatic long-memory scoring;
- full autonomous merge policies;
- heavy audit infrastructure as a prerequisite;
- broad Mentle integration in the live context path.

Early work should define:

- what files exist;
- who owns each file;
- what the prompt must read;
- what the prompt must output;
- what can be written automatically;
- what must become a proposal;
- what must require review;
- what must not be remembered.

Principle:

> Early Laputa does not compute subject state. It carries subject state. Early AutoDream does not prove memory truth. It proposes structured reflections for review.

## 4. GenericAgent as Trunk

The new architecture should continue to use GenericAgent as the main runtime trunk.

GenericAgent should keep owning:

- `AgentLoop`;
- `SessionManager`;
- `ContextBuilder`;
- tools and tool registry;
- subagent isolation;
- event bus;
- heartbeat and cron primitives;
- provider routing;
- `MemoryProvider` lifecycle boundary.

Laputa and AutoDream should not replace the GenericAgent runtime.

The right shape is:

```text
GenericAgent runs the agent.
Laputa owns the subject files.
AutoDream performs reflection work.
Heartbeat provides rhythm.
Mentle remains optional recall/tool memory.
```

## 5. Laputa as Subject File Layer

Laputa should absorb the subject-file management that should not be scattered across agent runtime modules.

Laputa should own:

- identity files;
- relationship files;
- commitment files;
- preferences or stable user-facing continuity files;
- `MEMORY.md`;
- `HISTORY.md`;
- daily reports;
- weekly reports;
- monthly reports;
- journal/reflection artifacts;
- evolution proposal inbox;
- subject-file changelog;
- report indexes and AAAK summaries.

GenericAgent should not directly scatter these paths through `AgentLoop`, `ContextBuilder`, or provider code. It should call a Laputa-facing boundary.

P0 can be file-only. It does not need a database, graph, vector index, resonance metric, or complex scheduler.

## 6. Heartbeat as Rhythm Source

Rhythm should use GenericAgent heartbeat rather than a new scheduler.

Current target:

```text
heartbeat tick
  -> RhythmPolicy checks daily/weekly/monthly due state
  -> emit or enqueue AutoDream work
  -> AutoDream produces report draft and proposals
  -> Laputa stores outputs
```

In P0, heartbeat may be represented only as an eligibility signal or postponed behind manual triggering. It should not block the main conversation path.

Daily, weekly, and monthly rhythm should be product shapes, not separate heavy subsystems.

## 7. AutoDream Redefined

AutoDream should be thin.

It should not own the whole memory architecture.

New definition:

> AutoDream is a background or manual reflection task that reads GenericAgent evidence and Laputa subject files, optionally recalls Mentle notes, and writes Laputa rhythm reports plus reviewable evolution proposals.

AutoDream should include four thin responsibilities:

```text
Trigger Adapter
  manual command / session-end eligibility / heartbeat rhythm

Context Collector
  read sessions, HISTORY.md, MEMORY.md, Laputa identity/rhythm files,
  and later optional Mentle recall

Prompt Runner
  run a structured reflection prompt

Result Writer
  write report drafts and proposals into Laputa
```

AutoDream should not own:

- authority memory;
- direct identity-file mutation;
- direct `MEMORY.md` mutation without review/policy;
- its own unrelated scheduler;
- a separate file universe outside Laputa;
- default Mentle injection;
- context compaction;
- broad tool access;
- complex scoring algorithms in P0.

## 8. AutoDream Naming Boundary

Naming must stay clear.

Use:

```text
Context Compaction
  current session prompt survival only

AutoDream
  background/manual memory reflection and organization task

Rhythm Evolution
  daily/weekly/monthly trigger strategy and product expression around AutoDream

Laputa
  subject file layer and report/proposal storage

Mentle
  optional external semantic notebook and recall tool
```

Important distinction:

```text
Diva Context Compaction != AutoDream
Diva AutoDream ≈ Claude AutoDream's background memory organization shape
Diva AutoDream extends that shape into Laputa subject continuity and rhythm reports
```

Claude Code AutoDream should not be described as ordinary context compaction. It is closer to background memory consolidation. Diva can borrow that shape:

- background execution;
- manual trigger;
- lock;
- checkpoint or timestamp;
- isolated/forked task context;
- non-blocking behavior;
- memory organization intent.

But Diva changes the target:

```text
Claude AutoDream:
  conversation memory consolidation

Diva AutoDream:
  GenericAgent sessions + Laputa files + optional Mentle recall
    -> rhythm report draft
    -> evolution proposal
    -> subject continuity refinement
```

Context compaction remains separate:

```text
trigger: context pressure
input: current session prompt history
output: compact working summary
writes: session checkpoint only
authority: none
```

## 9. Suggested AutoDream Scopes

AutoDream can later support scopes without becoming multiple systems:

```text
autodream run --scope session
autodream run --scope daily
autodream run --scope weekly
autodream run --scope monthly
autodream run --scope identity
```

Meaning:

- `session`: closest to Claude AutoDream; organize one or several recent sessions.
- `daily`: write or update a daily reflection/report draft.
- `weekly`: summarize week-level patterns, projects, repeated issues.
- `monthly`: higher-level continuity review.
- `identity`: high-risk identity or relationship proposal; requires strong review.

P0 does not need all scopes. The naming simply avoids future confusion.

## 10. Evolution Proposal P0

The P0 autonomous evolution loop should use one simple proposal shape rather than many candidate types.

Suggested P0 object:

```text
EvolutionProposal
  id
  created_at
  source_session_ids
  proposal_type: memory_patch | journal_note | learning_note | identity_patch
  summary
  evidence_excerpt
  target_file
  proposed_patch
  risk_level
  status: proposed | approved | rejected | edited | applied
```

P0 should avoid full `EvidenceRef` complexity. It can use:

- source session IDs;
- short evidence excerpts;
- optional turn ranges;
- links to Laputa report files.

Later versions can expand this into richer evidence references.

## 11. Review and Authority

Model output should not directly become authority memory.

Rules:

- AutoDream can write report drafts.
- AutoDream can write proposals.
- AutoDream can write non-authority intermediate notes.
- AutoDream should not silently rewrite `MEMORY.md`.
- AutoDream should not silently rewrite identity files.
- High-risk changes require user review.
- Approved changes should produce changelog entries.

Authority order:

1. Laputa authority files and `MEMORY.md`.
2. Laputa changelog and proposal review records.
3. Full rhythm reports and journal artifacts.
4. Mentle indexes, notes, and temporary work-memory entries.

## 12. Mentle Role

Mentle should be optional and external by default.

Core decision:

> Mentle is an external semantic notebook and recall tool. It can help Diva remember, but it should not be Diva's always-active mind or the authority source for subject continuity.

Mentle content should not enter daily conversation context by default.

Daily use:

```text
conversation
  -> harness/prompt decides whether recall is needed
  -> optional Mentle search/read
  -> selected results enter only the current task context
```

Mentle should expose simplified wings:

```text
mentle.search(query, filters)
mentle.read(id)
mentle.write(note)
mentle.update(id, patch)
mentle.delete_or_archive(id)
```

Optional later:

```text
mentle.link(source_id, target_id, relation)
```

Mentle is appropriate for:

- project research summaries;
- technical conclusions;
- reusable procedures;
- command notes;
- troubleshooting records;
- project decisions and constraints;
- open questions;
- report indexes;
- AAAK summaries;
- AutoDream intermediate material;
- major-task temporary work memory;
- user-explicit memory notes.

Mentle is not appropriate as an unreviewed sink for:

- casual one-off chat;
- unstable emotional fragments;
- inferred personality judgments;
- sensitive private information without explicit authorization;
- raw model speculation about the user;
- identity conclusions that belong in Laputa after review.

## 13. Mentle in AutoDream

AutoDream may later deliberately call Mentle.

Future shape:

```text
AutoDream / Rhythm Evolution
  -> read recent sessions
  -> read Laputa files
  -> optionally call Mentle recall for related history, project notes, and report indexes
  -> generate daily/weekly/monthly report
  -> generate EvolutionProposal records
  -> write proposals to Laputa inbox
  -> optionally write searchable intermediate notes to Mentle
  -> update authority files only after review/policy approval
```

Mentle can suggest and retrieve. It must not authorize durable changes.

## 14. Mentle as Temporary Work Memory

Mentle can serve as temporary memory during major work.

Suggested shape:

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

Lifecycle:

```text
task start
  -> recall work_memory
  -> load selected parts into current context

task progress
  -> update work_memory when durable intermediate state appears

task end
  -> AutoDream / Laputa decides what becomes report, proposal, archive, or long-term memory
```

This improves long-work continuity without keeping every temporary note in active prompt context.

## 15. Daily, Weekly, Monthly Reports

Daily, weekly, and monthly reports should live in Laputa.

They are not just reports. They are rhythm artifacts that can support future memory proposals.

Suggested output:

```text
RhythmReport
  period
  narrative_summary
  important_events
  project_updates
  open_questions
  memory_proposal_refs
  non_memory_notes
  source_refs
```

Retrieval should be staged:

```text
1. Search report indexes / AAAK summaries.
2. Select relevant daily/weekly/monthly reports.
3. Read full report files.
4. If needed, read original sessions or source material.
```

Mentle may index report metadata and AAAK summaries, but the full report entity remains in Laputa.

## 16. AAAK Summaries and Report Indexing

Reports should have compact recall handles.

Each report can produce:

- a human-readable full report;
- an index entry;
- an AAAK-style compressed summary;
- links to source sessions or source files.

Purpose:

- avoid injecting all reports into context;
- allow low-cost recall;
- let Diva decide whether to read the full entity;
- give Mentle better search targets without owning authority.

## 17. Obsidian Direction

Future Obsidian integration fits the file-first design.

Suggested split:

```text
Laputa files
  human-readable authority and rhythm files

Obsidian
  human-facing linked note workspace / knowledge garden

Mentle
  machine-facing semantic index and recall engine
```

Mentle may index Obsidian and Laputa files. Authority material should remain readable, editable, and versionable.

## 18. Context Compaction Priority

Context compaction is important, but it is not the current main goal.

It should be treated as:

```text
session-local survival mechanism
```

It answers:

> How does the current conversation continue safely when the prompt becomes too large?

It should not:

- write `MEMORY.md`;
- create long-term identity facts;
- become AutoDream;
- drive rhythm reports;
- become the only source of evidence for memory updates.

P0 autonomous evolution should not block on full context compaction.

However, early hooks are useful:

- context budget monitor;
- prompt history policy;
- session compaction checkpoint;
- future ability to cite a compaction summary as secondary evidence only.

## 19. GenericAgent Original vs New Architecture

GenericAgent original direction is a general learning agent engineering framework.

The new direction is Diva subject continuity on top of GenericAgent.

Differences:

| Area | GenericAgent original | New Diva direction |
|---|---|---|
| Goal | general agent learning and memory | Diva subject continuity and rhythm reflection |
| Memory center | MemoryProvider and layered memory | Laputa files as authority, MemoryProvider as runtime boundary |
| Learning | consolidation/prefetch/provider lifecycle | prompt-governed reports and proposals |
| Storage style | generic layers and backend abstractions | identity files, reports, proposal inbox, changelog |
| Mentle | possible deep hybrid memory provider | optional external recall/notebook tool |
| Rhythm | infrastructure | daily/weekly/monthly subject rhythm |
| AutoDream | not central as Diva product | thin reflection job over GenericAgent + Laputa |

## 20. GenericAgent Capabilities Not Yet Fully Covered

The new architecture should not forget these GenericAgent disciplines:

1. `MemoryProvider` lifecycle:
   - `system_prompt_block()`;
   - `prefetch()`;
   - `sync_turn()`;
   - `on_session_end()`.

2. Online prefetch:
   - current message may need selective recall;
   - future `RecallPolicy` can decide no recall, Laputa render, Mentle search, or report index lookup.

3. Turn/session consolidation:
   - should be downgraded from direct memory write to proposal/history append.

4. L0-L4 layering:
   - should be mapped into Laputa rather than discarded.

5. Tool and worker isolation:
   - AutoDream should not inherit the full daily tool registry.

6. ContextBuilder rendering:
   - Laputa prompt rendering needs explicit Always / Relevant / Archive behavior.

7. Failure downgrade:
   - Mentle failure should not block conversation;
   - AutoDream failure should not block conversation;
   - heartbeat failure should not block AgentLoop;
   - Laputa read failure should degrade predictably.

8. Configuration switches:
   - Laputa enablement;
   - rhythm enablement;
   - AutoDream manual/heartbeat enablement;
   - Mentle tool-call enablement;
   - Mentle default context injection must remain false.

## 21. L0-L4 Mapping into Laputa

Keep GenericAgent's layering discipline but use Diva/Laputa names.

Suggested mapping:

```text
L0 active context
  current prompt, current task state, recent conversation

L1 index
  report indexes, AAAK summaries, Mentle handles, lightweight recall cards

L2 distilled memory
  MEMORY.md, identity files, relationship files, commitment files

L3 reflection
  daily/weekly/monthly reports, journal artifacts, AutoDream outputs

L4 raw evidence
  sessions, logs, source files, transcripts
```

This prevents `MEMORY.md` from becoming a raw archive while preserving Diva naming.

## 22. Prompt Rendering Boundary

Laputa should not dump all files into prompts.

Suggested rendering layers:

```text
Always
  identity compass, relationship compass, standing commitments, active project compass

Relevant
  current-task-related memory snippets, report indexes, selected Mentle recall results

Archive
  full reports, old sessions, raw logs, unselected notes
```

`ContextBuilder` should consume rendered prompt-safe material, not manually read random Laputa files.

## 23. AutoDream Worker Tool Profile

AutoDream should use a restricted tool profile.

Allowed early:

- read recent sessions;
- read Laputa files;
- write Laputa report drafts;
- write proposal inbox entries;
- append changelog after approved apply;
- optional Mentle search/read/write for non-authority notes.

Disallowed or disabled by default:

- shell;
- spawn;
- broad filesystem mutation outside Laputa;
- cron self-scheduling;
- network;
- destructive tools;
- direct identity or `MEMORY.md` mutation without review/policy.

## 24. P0 Implementation Shape

P0 should stay small.

Suggested P0:

```text
manual AutoDream run
  -> collect recent sessions + Laputa identity/MEMORY/HISTORY files
  -> run structured prompt
  -> write report draft and EvolutionProposal records
  -> list proposals
  -> approve/reject/apply proposals
  -> apply writes authority files and changelog
```

P0 may skip:

- full rhythm automation;
- full context compaction;
- full EvidenceRef schema;
- full audit event stream;
- Mentle integration;
- Obsidian integration;
- GUI inbox;
- automatic merge.

## 25. P1 / P2 Direction

P1:

- heartbeat-based daily/weekly/monthly eligibility;
- report indexes;
- AAAK summaries;
- simple recall policy;
- optional Mentle recall for AutoDream;
- proposal inbox improvements;
- basic worker locks/checkpoints.

P2:

- richer evidence refs;
- source capsules if still useful;
- full audit stream if needed;
- Obsidian integration;
- GUI review;
- lower-risk automatic proposal application under strict policy;
- deeper Mentle indexing and pattern discovery.

## 26. Current Non-Goals

Do not do in the first implementation pass:

- make Mentle default context;
- make AutoDream directly mutate authority memory;
- build a complex resonance algorithm;
- build a full graph memory system;
- treat context compaction as AutoDream;
- treat daily/weekly/monthly reports as direct memory truth;
- let GenericAgent runtime own Laputa files directly;
- make `MEMORY.md` the raw archive;
- require full audit infrastructure before the first proposal loop works.

## 27. Final Principle

Use this principle for future design and implementation review:

> GenericAgent keeps Diva running. Laputa keeps Diva continuous. AutoDream reflects and proposes. Rhythm decides when to reflect. Mentle is the notebook Diva can open. Context compaction keeps the current conversation alive. Authority changes require files, review/policy, and changelog.

