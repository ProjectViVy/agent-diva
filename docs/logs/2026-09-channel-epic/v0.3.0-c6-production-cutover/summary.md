# C6 production cutover summary

## Outcome

The six active platforms now have a real Manager-owned production runtime. Manager constructs the
shared attachment authority, adapter registry, bounded pacing lanes, listener supervisors, and the
Fabric ingress consumer. Telegram, Discord, Feishu, DingTalk, Email, and QQ no longer instantiate
the retired `ChannelHandler` tree.

The desktop channel settings surface now reads runtime registration and health from
`GET /api/channels/runtime`. Channel updates wait for runtime reconfiguration; a failed candidate
construction restores the previous configuration and runtime. Incomplete credentials remain
loadable so the adapter health diagnosis can be shown without taking down the gateway. Context
compaction uses a dedicated runtime-control command instead of synthesizing a chat message.

Retired channel source/config/features and the old chat/SSE/Tauri registrations were removed. The
remaining Agent message queue is fixed-capacity and has one egress owner; callback subscribers and
unbounded ingress/egress were deleted.

## Completion boundary

This delivery makes the new channels effective on `feat/channel-epic-c6`, but does not close C6.
`InboundMessage` and `OutboundMessage` still exist inside AgentLoop and are converted at the native
egress boundary. The frozen architecture requires their physical deletion. Post-cutover real
desktop/platform acceptance and the Rust 1.80 probe are also still required before an atomic merge
to `dev`.

## Impact

- Runtime: Manager, Fabric admission, native adapter lifecycle, bounded egress.
- Configuration: only the six active channel schemas remain; incomplete enabled configurations stay
  loadable so runtime health can report actionable credential errors.
- Desktop: runtime-backed status, six-channel-only setup, dedicated compaction command.
- Build/CI: `channel-clean-break-check` detects retired production registrations and unbounded bus
  regressions.
