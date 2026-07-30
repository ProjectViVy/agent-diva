# GMH-24B Acceptance

1. Start with `memory.authority_mode = "legacy"` and confirm legacy behavior.
2. Import typed records, select `shadow`, restart, and confirm answers are
   unchanged while payload-free comparison metrics are emitted.
3. Damage or mismatch a disposable typed store and confirm Manager reports
   `degraded` with a stable reason and does not silently use a different mode.
4. Select `typed`, restart, and confirm Recall v2 supplies the prefetch block.
5. Restore `legacy` to verify configuration rollback remains available before
   GMH-24C write cutover.
