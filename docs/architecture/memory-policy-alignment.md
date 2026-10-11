# Unified context pipeline: current memory policy

Revision: 2026-10-11. This is DIVA's documentation alignment with the owner's
latest decisions in [VIVY #58](https://github.com/ProjectViVy/agent-vivy/issues/58)
(**Unified context pipeline: memory enhancement, Recap-only ACTMEM and editable UI**)
and [VIVY #57](https://github.com/ProjectViVy/agent-vivy/issues/57).
It specifies selected future behavior; documentation alignment does not establish
implementation, product acceptance, a dependency repin or a data migration.

## Ownership and pilot

The foundation belongs across VIVY. LIFE is the first pilot and is enabled by
default under the selected policy; coding is excluded from phase 1. Use the
existing `Service.Run`, Journal, policy/admission and context assembly path.
Do not add another runtime, memory authority, compaction system or LIFE pipeline.

Backend CRUD/search stays behind the same capability-aware abstract interface.
BML, Mentle and other providers own storage, indexing and returned content;
VIVY owns authorized query scope, admission and context budgets. Validate BML
first. The owner validates Mentle through DIVA's existing Garden/Mentle route.
This authorizes neither a DIVA backend switch nor dual writes or data migration.
The existing BML completion projection is not a full transcript; raw evidence
must not be inferred from that projection. Do not relabel DIVA's Mentle evidence
as BML validation or substitute a second BML database for its selected backend.

## Recap and retirement

All persistent sessions of one authorized subject share **one subject-level,
free-form Recap**, organized by AI. ACTMEM's selected active policy is Recap-only;
there is no separate Recap authority for each session and no prescribed outline.
Subject binding is authoritative host state, never an arbitrary UI/model ID.

Consolidation uses the prior committed Recap plus the retiring raw conversation
segment to produce the next committed Recap. Commit that Recap before unloading
the old raw segment from active model context. Retain persistent raw evidence;
unloading context does not delete Journal, archived/raw sources or provenance.
Consolidation and human edits require revision conflict handling. A stale
revision must conflict or be reconciled explicitly; silent last-writer-wins
replacement is forbidden. Exact subject keys, transaction boundaries, conflict
UX and recovery protocol remain design work.

Memory may enter the existing context path under authorized scope, admission
and measured budgets. Retrieval, maximum context and consolidation budgets are
experimental; fixed numbers in older implementations/evidence are not selected
product defaults. **WORLD remains tool-only: automatic injection is prohibited.**
The old Pulse/Recap/Work and blanket tool-only ACTMEM requirements are superseded
only for the selected policy. Existing Work, reconcile, capture, archive and
recovery compatibility remains planned; no current storage or code deletion is
authorized by this document.

## Human controls and explicit tools

Provide a dedicated UI that lets the human inspect the full Recap, edit it and
delete it, with scope and revision/conflict state visible. Ordinary Recap edits
are editable memory, not permanent pinning. “Must not be forgotten” directs the
human to the existing Laputa frozen-layer / Frozen Core human-controlled
workflow. Never automatically write Mission, Dream or frozen persona for this
request. Existing frozen-session and governed human authority stay intact.

ACTMEM inspect, update and delete Agent tools are required, but normally hidden.
Explicit need activates them through deferred discovery; no always-on schema
loading or autonomous maintenance is selected. Discovery mechanics, activation
lifetime and authorized tool contracts remain to be designed. Existing human
control actions do not prove model-callable ACTMEM tools: the historical v3.1
check found those read tools absent.

Deletion is explicit and targeted. UI ACTMEM delete removes active Recap,
state/projection only; it must not claim archived/raw evidence was erased.
Authorized backend delete tools affect only selected resources and never infer
cascading cleanup. Raw/archive erasure requires its own explicitly authorized
resource operation. Tests and UI copy must distinguish these outcomes.

[VIVY #57](https://github.com/ProjectViVy/agent-vivy/issues/57) role-only workers
receive only role-required capabilities and admitted context. They do not
inherit a default persona, unrestricted history or memory permission. Deferred
tool discovery does not grant authority.

## Validation and historical evidence

DIVA origin/main at reconciliation is
`0a64d1ce152440f1f63d4224a155c4616d2aff5b`. Existing source pins and artifact
hashes are unchanged. Laputa [PR #5](https://github.com/ProjectViVy/laputa/pull/5)
merged at `4b2bec2`; VIVY [PR #46](https://github.com/ProjectViVy/agent-vivy/pull/46)
merged at `59a673ac`; VIVY [PR #48](https://github.com/ProjectViVy/agent-vivy/pull/48)
merged at `e67e6ff`, including the BML-FIRST-INIT-CANCEL fix.
VIVY exact commit `ce33bc8776b9f52ab13bed5f2cdaf8ebad01f666` has
[CI run 38095646661](https://github.com/ProjectViVy/agent-vivy/actions/runs/38095646661)
with all four checks successful, including the full supported DIVA memory lane.
Current VIVY main has also retired `sdk/host/v1` and SDK `shared`/`go-host`
targets, while DIVA's unchanged pinned desktop/build consumer still references
those older seams. Define the replacement/compatibility path before upgrading
DIVA; this document does not authorize a runtime replacement or source repin.
The successful supported memory lane is not installed-DIVA build evidence.
These integration facts close the former dependency/CI follow-ups, not the new
Recap implementation or installed-product acceptance.

Developer recall, correction/deletion and six crash-cut proofs are real and
remain preserved in the [memory-loop evidence index](../plans/diva-next/memory-loop/README.md).
Windows installed-product, real-model, unknown/partial-effect recovery, backend
recovery and workspace isolation matrices remain unclosed. Historical v3.1's
no-ACTMEM-injection result remains evidence for its exact earlier baseline;
it is not the new V19 acceptance rule. Story status is still maintained only in
that index; existing sample counts and candidate binding rules remain intact.

## Open design decisions

- Define authoritative subject identity and isolation across persistent sessions,
  profiles and supported workspaces without inventing a second memory store.
- Define Recap revision/CAS, merge/retry decisions, human conflict UI and the
  commit-before-unload crash/recovery contract, including concurrent writers.
- Define full Recap UI and explicit active-delete behavior, plus how later raw
  consolidation respects an active deletion without claiming raw erasure.
- Define capability-aware backend contracts and BML-first tests; record the
  owner's Mentle-through-DIVA validation without selecting a backend switch.
- Define deferred tool discovery and inspect/update/delete authority; no
  automatic maintenance or permanent schemas are implied.
- Measure admission/retrieval/consolidation budgets and role-worker permissions
  through the existing pipeline; no fixed budget numbers are settled.
- Retain compatibility for existing Work/reconcile/capture until separately
  authorized implementation resolves it; re-estimate affected story work.
