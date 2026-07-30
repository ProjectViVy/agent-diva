# E7 Governance Observability

## Outcome

GMH-42 is closed with a payload-free governance metric surface. Laputa now
tracks:

- new governance decisions and denials;
- endpoint decision latency totals/maxima;
- proposal-to-human-decision wait totals/maxima;
- stale, expired, version-conflicted or already-consumed receipts;
- successful typed applies and typed rollbacks;
- complete and incomplete correlation chains.

Manager health embeds the snapshot under `memory.governance_metrics`. Every
field is a counter or latency aggregate; no proposal patch, Memory content,
receipt payload, user prompt or secret is exposed.

Typed apply correlation covers governance request, proposal, changelog, audit
and rollback request identifiers. Typed rollback requires changelog, audit and
proposal linkage. Idempotent completed apply replays return before incrementing
the operation metric again.
