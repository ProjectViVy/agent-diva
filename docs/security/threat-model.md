# Memory and Governance Threat Model

## GMH-21 normalized Memory boundary

Normalized records are transport and migration contracts, not a new authority
store. In a Laputa workspace, only applied owned sections may be adapted with
`applied_authority`; when `.laputa` is absent, the explicitly selected legacy
Markdown owner may receive that trust. A broken Laputa workspace remains
degraded.

Primary threats and controls:

| Threat | Control |
| --- | --- |
| Prompt content impersonates instructions | Render only through a length-delimited escaped data block; adapters never execute or interpret content |
| Tool, session, AutoDream, user, or compaction data claims authority | Provenance and trust are independent; validation rejects non-owner sources marked `applied_authority` |
| Cross-workspace record reuse | Tenant/workspace scope is mandatory and callers can require an exact workspace match |
| Content or migration artifact tampering | SHA-256 binds source content, records, and migration manifests; changed input under the same migration ID conflicts |
| Pending or unsupported Laputa data enters Memory | Only `Owned` applied sections produce records; non-owned sections produce findings and no records |
| Migration overwrites authority | Artifacts use an explicit isolated root; rollback deletes only manifest-listed outputs and never source files |
| Unknown schema fields silently gain trust | Unknown Laputa top-level fields become integrity warnings; unknown contract enums fail record validation |
| Sensitive deletion reason leaks content | Tombstones contain only a reason digest, actor, timestamp, and target ID |

GMH-21 does not route records into prompts or production reads. GMH-22 owns
recall filtering and rendering; GMH-23 owns proposal/apply writes; GMH-24 owns
shadow comparison and cutover gates.

## GMH-22 Recall v2 boundary

Recall v2 treats retrieval as candidate discovery, never as authorization.
Default prompt selection requires applied authority provenance and rejects
restricted or unknown sensitivity.

| Threat | Control |
| --- | --- |
| High search relevance promotes untrusted data | Relevance affects ranking only; trust is an independent allowlist filter |
| Cross-tenant/workspace/session recall | Exact scope checks run before scoring |
| Expired, tombstoned, or replaced data returns | Temporal, tombstone, supersedes, and digest-dedup stages precede budgeting |
| Candidate floods consume context | Candidate limit, deterministic ordering, and hard token budget; records are skipped rather than truncated |
| Recall text escapes its prompt boundary | GMH-21 length-delimited escaping is the only Recall v2 renderer |
| Diagnostics leak Memory content | Trace and shadow reports contain IDs, digests, scores, token counts, and reason enums only |
| Retrieval outage reuses stale data | Typed degraded result contains no prompt block and no cached fallback |

Recall v2 remains shadow-capable only in GMH-22. The current production prefetch
path is unchanged until GMH-24 validates comparison metrics and cutover.

## GMH-23A Embedded Laputa and Mentle clean-break amendment

The target Diva-local Memory store is Embedded Laputa: profile-local typed
SQLite records with FTS5 retrieval and Gateway-only mutation. Mentle/MenPalace
is not an allowed provider, fallback, migration reader, or optional backend.

| Threat | Control |
| --- | --- |
| Native Mentle/LLVM dependency returns transitively | Deletion-proof manifest, lockfile, source, CI and active-doc scans |
| Old Mentle database is silently trusted | No runtime reader or online migration; any future import requires a separately approved offline converter |
| Broken Embedded Laputa falls back to Markdown/Mentle | Typed degraded state with no fallback authority |
| SQLite mutation bypasses governance | Gateway-only mutation with proposal digest, valid receipt, transaction and audit correlation |
| FTS results self-promote into authority | Retrieval remains transient; only applied records may enter authority projections |
| Cross-profile leakage | Profile/workspace partition keys are mandatory in storage, retrieval and API contracts |
| Long-lived dual write produces two truths | Shadow read only; bounded read/write cutover flags with mandatory removal dates |

GMH-23 stage 3 and the previous GMH-24 plan are paused until the storage and
retrieval prerequisites GMH-23A through GMH-23C pass.
