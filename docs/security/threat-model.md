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
