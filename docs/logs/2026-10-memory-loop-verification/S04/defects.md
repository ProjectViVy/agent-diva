# MEM-S04-01 — User-only facts lost at terminal capture

Severity P0; OPEN. Diagnostic sample V05; not the full V05–V09 acceptance matrix.

The real App and generated DIVA recipe pass the synthetic user text `synthetic random fact not repeated in assistant response` into the actual DeepSeek-compatible HTTP request. The scripted provider returns only `收到`. Actual Garden ingestion and Mentle canonical source contain only `收到`; SourceRole is absent. Request, source, run/sequence/ingestion/record/revision snapshots are retained here. This literal is synthetic, not a random uniqueness/cross-profile proof.

Root cause: VIVY internal/runtime/mapper.go completedEvent projects the last assistant summary (bounded at 8 KiB); internal/runtime/cognitive_service.go CognitiveCaptureProvider.ObserveRunWithReceipt passes payload.Summary as source content. Facts stated only by the user never enter this durable source.

Expected repair: use trusted run-specific durable user/assistant sources with role and truncation semantics, preserve terminal/ingestion idempotency and recovery. Do not invent SourceRole in the observer, append a test assertion's fact, or build a parallel memory authority. Implement S04 only after its predecessor gates; validate V05–V09 including role, long inputs, isolation and replay. No product source repair is included in this cut.
