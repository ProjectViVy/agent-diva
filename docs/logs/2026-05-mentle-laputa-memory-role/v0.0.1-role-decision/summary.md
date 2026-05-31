# Summary

## Change

Added `docs/dev/genericagent/mentle-laputa-memory-role-decision.md`.

The document records the accepted architecture direction for Mentle after simplifying autonomous evolution around GenericAgent, Laputa files, heartbeat rhythm, and prompt-governed decisions.

## Key Decisions

- Mentle is not default prompt context.
- Mentle is an optional external semantic notebook and tool-call memory.
- Laputa owns subject continuity files, identity files, rhythm reports, proposals, and authority memory artifacts.
- AutoDream may later call Mentle for recall, but Mentle does not authorize durable memory changes.
- Daily, weekly, and monthly reports should have indexes and AAAK summaries for staged recall.
- Mentle can serve as temporary work memory during major tasks.
- Obsidian can later become a human-facing linked note surface over the same file-first direction.

## Impact

This reduces memory architecture complexity by keeping authority in files, keeping Mentle optional, and turning daily memory usage into harness/prompt-governed tool calls rather than automatic context injection.

