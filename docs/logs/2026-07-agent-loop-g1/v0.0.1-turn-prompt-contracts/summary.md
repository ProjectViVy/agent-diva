# Summary

Established internal governed-turn contracts for admission, policy snapshots,
prepared context, model outcomes, tool policy, and finalization. Tool policy is
refreshed from the active Plan immediately before the existing executor seam.

Moved main-runtime LLM prompts into subsystem-local prompt modules and converted
compaction, consolidation retry, context boundaries, Plan, title, cron, and
downstream compaction labels to English. Core and GUI Plan validation now accept
English and legacy Chinese section headings.

The follow-up extraction completes G1.3–G1.6. Admission and execution hydration,
provider context preparation, bounded sampling and streamed response handling,
the pre-executor policy seam, and final persistence/events now delegate to the
`turn` modules.

The G1.6 closeout moves attachment/security/vision handling, execution-context
hydration, compaction/history preparation, and recall prefetch into
`turn/context.rs`; the complete registry-backed tool lifecycle into
`turn/tool_step.rs`; and Soul/Plan demultiplexing plus persistence/outbound
construction into `turn/finalize.rs`. The sole runtime path is retained and
`process_inbound_message_inner` is reduced from 865 to 350 lines.
