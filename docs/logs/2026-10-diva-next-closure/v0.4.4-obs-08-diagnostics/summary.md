# OBS-08: Bounded diagnostics and honest GUI persistence (v0.4.4)

Console "Diagnostics" panel: bounded `diagnostics/logs` reads (runtime +
GUI families; source/date/level/query verbatim, server-cursor paging,
gap/has_more/truncated surfaced as-is) plus a `GuiDiagnosticRecorder` —
bounded in-memory queue (<=1000 records / <=1 MiB, drop-oldest counted),
sanitized allowlisted records (speech secrets/text/audio/body/URLs
redacted before recording), batches <=50 records / <=256 KiB through
`diagnostics/gui/append`, single-flight drain. Persistence accounting is
honest: only `accepted` counts as persisted; failed or ambiguous writes
park the batch as unconfirmed — never retried, never parsed from error
text. `recordGuiDiagnostic` is the DN-6C entry point.
