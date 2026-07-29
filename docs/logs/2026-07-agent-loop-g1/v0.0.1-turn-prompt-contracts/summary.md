# Summary

Established internal governed-turn contracts for admission, policy snapshots,
prepared context, model outcomes, tool policy, and finalization. Tool policy is
refreshed from the active Plan immediately before the existing executor seam.

Moved main-runtime LLM prompts into subsystem-local prompt modules and converted
compaction, consolidation retry, context boundaries, Plan, title, cron, and
downstream compaction labels to English. Core and GUI Plan validation now accept
English and legacy Chinese section headings.

The follow-up extraction completes G1.3–G1.5. Admission and execution hydration,
provider context preparation, bounded sampling and streamed response handling,
the pre-executor policy seam, and final persistence/events now delegate to the
`turn` modules. `process_inbound_message_inner` remains the only coordinator and
was reduced from about 1,400 lines to 865 lines.

G1.6 remains open against the broader architecture target: context compaction,
tool-result orchestration, and Plan report demultiplexing still keep the
coordinator above the approximate 500-line review threshold.
