# GMH-24B Shadow and Read Cutover Summary

Manager runtime now constructs Memory authority from the strict configured
mode. `legacy` retains the Markdown provider. `shadow` runs Embedded Laputa
Recall v2 beside the legacy prefetch result and returns only the legacy result.
`typed` returns the typed Recall v2 prompt block.

Shadow metrics and comparisons contain counts, timings, token usage, record
identifiers, and digests without raw Memory payload. Typed-store open,
workspace, schema, FTS, and integrity failures produce an explicit degraded
boundary instead of a silent legacy fallback.

The Manager health response exposes the authority mode, typed-store status,
revision, record/tombstone counts, and a stable redacted failure reason.
