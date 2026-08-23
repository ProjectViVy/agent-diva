# Record neuro-link as a reserved heavyweight channel

- Date: 2026-08-23
- Branch: `dev`
- User decision: neuro-link is intentionally reserved as a future
  heavyweight channel, not a missing `channel_statuses` row.

## Changes

- `NeuroLinkConfig` / `ChannelsConfig.neuro_link`: reserved-future comments.
- `channel_statuses`: document why `neuro-link` is omitted; trailing note
  in the status vec.
- `neuro_link.rs` module docs: same contract; repaired garbled protocol
  arrows in the existing header.
- TODOLIST: `CHANNELS-STATUS-COVERAGE` reframed to
  `NEURO-LINK-HEAVYWEIGHT-CHANNEL` (pending start) and moved from
  频道遗留 into **EPIC 新启动**. Do not fill ready/missing_fields as a
  telegram-style gap.

## Not done

- No config-status / GUI / runtime redesign of neuro-link.
- No GUI file edits (style-phase2 lock).
