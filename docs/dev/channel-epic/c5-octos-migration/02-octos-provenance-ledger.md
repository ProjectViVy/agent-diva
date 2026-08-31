# Octos provenance ledger

Reference snapshot: `5ea987813de4fd2afdd1d78f2106ad2868f0d923`, tag `v2.0.3-rc.9`,
Apache-2.0. This ledger describes design sources; it is not permission to copy entire modules or
dependencies. Any later non-trivial direct adaptation must add the exact source symbol and preserve
required attribution.

| Area | Octos source | Decision | Diva treatment |
| --- | --- | --- | --- |
| Long-running channel shape | `octos-bus/src/channel.rs::Channel` | adapt | Keep Diva `ChannelAdapter`; reject all default-success optional methods |
| Message IDs and editing | `send_with_id`, `edit_message`, `finish_stream` | adapt | Closed `ChannelCommand` plus truthful receipt/capability checks |
| Thread binding | `stream_reporter.rs` bound edit/finalize paths | port invariant | Always use envelope correlation/address; never sticky per-chat state |
| Chunking | `ChannelManager::start_all`, coalesce helpers | reject runtime | Existing C2 pacing owns chunking, retry safety, and partial receipts |
| Telegram | `telegram_channel.rs` media, typing/listening, send ID, edit/delete, health | port behavior | Reuse current teloxide transport and Diva typed attachments |
| Discord | `discord_channel.rs` send ID, edit/delete, reactions, embeds | port behavior | Keep current HTTP/WS transport; do not introduce Serenity merely to match Octos |
| Feishu | `feishu_channel.rs` reply endpoint, media, edit/delete, mock server | port behavior | Preserve Diva Protobuf WS/ACK/heartbeat/card path and add typed evidence |
| DingTalk | `dingtalk_channel.rs` webhook/session behavior | retain-diva/reject fallback | Preserve Diva Stream WS and media; borrow only independently useful parsing/test patterns |
| Email | `email_channel.rs` parsing, Message-ID, subject/reply tests | adapt | Preserve consent, spawn-blocking boundaries, attachments, bounded polling |
| QQ | `qq_bot_channel.rs` C2C/group, gateway session, resume, dedup | port behavior | Preserve native-TLS requirement; add typed media and receipt proof beyond Octos gaps |
| Manager/bus | Octos `ChannelManager`, `BusPublisher` | reject | Existing Fabric, Registry, pacing, supervisor, Agent admission remain authoritative |
| Default health/no-op | `Channel::health_check` and optional defaults | reject | Unsupported is typed; `Unknown` cannot be advertised as `ReliabilityHealth` proof |

## Known Octos limits that must not leak into the target

- The generic trait permits silent success for unsupported typing/edit/delete/reaction operations.
- Listener exits are logged by the manager but do not provide Diva's shared supervised restart.
- QQ media is incomplete relative to the C5 target.
- DingTalk is a webhook-oriented implementation and is weaker than Diva's existing Stream path.
- Octos workspace/dependency choices are not the Agent Diva MSRV authority.

## Provenance update rule

Each platform implementation commit must extend this ledger if it adapts a non-trivial Octos
algorithm. The entry must name source file, symbol, Octos SHA, local destination, transformation,
and the test that proves Diva semantics. Pure protocol use or independently written code should be
identified as such rather than falsely attributed as a code port.
