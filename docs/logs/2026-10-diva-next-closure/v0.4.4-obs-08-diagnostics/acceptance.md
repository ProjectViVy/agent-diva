# Acceptance — OBS-08

- [x] Read path: source/date/after/limit/level/query verbatim;
      gap/has_more/truncated shown as producer flags.
- [x] Append path: batches <=50 records / <=256 KiB (facade rejects
      larger); success ack counts only `accepted`.
- [x] Ambiguous/partial append -> whole batch unconfirmed, no retry, no
      error-text parsing (partialAppendUnknownNoRetry).
- [x] Queue overflow drops oldest with visible count; disconnect shows
      loss without blocking chat (recorder is fire-and-forget).
- [x] Diagnostic errors cannot feed the recorder (no self-capture;
      readerErrorNoRecursion).
- [x] Speech diagnostics exclude secret/text/audio/body/full URL before
      recording (speechSentinelRedacted).
- [x] `recordGuiDiagnostic` exported as the DN-6C entry point.
- [ ] Native filesystem rotation/reopen smoke: deferred (OBS-09/dev).
- [ ] Owner acceptance: PENDING.
