# Verification

- The focused vertical Manager test passes.
- The complete Laputa crate suite passes: 14 unit groups and all integration
  suites, with only the two explicit 10k performance tests ignored by default.
- The Manager library suite passes twice consecutively: 76/76 each run.
- Memory boundary focused tests pass, including shadow missing-store fail-closed.

Assertions include source run provenance, governed apply, typed Recall, one
payload-free feedback event, rollback success, and zero recalled records after
rollback.

No external provider, desktop key, or manual desktop action was used.
