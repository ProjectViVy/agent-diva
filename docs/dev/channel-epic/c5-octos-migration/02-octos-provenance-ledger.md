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

## C5-P2 endpoint-level provenance index

The following index is the minimum source map for the six independently scanned reports. It is an
index, not a license to copy modules wholesale; every implementation commit must add its exact
destination symbol and transformation.

| Channel | Octos source anchors | DIVA source anchors | Migration boundary |
| --- | --- | --- | --- |
| Telegram | `telegram_channel.rs::download_telegram_file`, `send_html_with_fallback`, `parse_inline_keyboard`, polling/event loop, `send_with_id`, edit/delete, health | `src/telegram.rs::handle_message`, polling branches, `send`, `edit_message`, `delete_message`, `request_stop_via_api` | Port wire semantics; adapt to typed attachment/receipt; retain DIVA commands/governance; remove hardcoded stop transport in C5-I |
| Discord | `discord_channel.rs::Handler::message`, `send_with_id`, edit/delete, reaction parser, embed, `MessageDedup` | `src/discord.rs::handle_gateway_message`, `fetch_gateway_ws_url`, `start_typing`, send path | Keep HTTP/WS; port REST payload/error/reaction/embed behavior; reject Serenity/runtime import |
| Feishu | `feishu_channel.rs::base_url_for_region`, webhook crypto, `download_feishu_media`, upload image/file, send/reply/edit/delete | `src/feishu.rs::get_access_token`, WS frame/event parser, `fetch_image_marker`, `add_reaction`, send/reply | Port region/webhook/media; retain protobuf WS, reaction seen, card/table |
| DingTalk | `dingtalk_channel.rs::verify_dingtalk_signature`, session cache, webhook target/send | `src/dingtalk.rs::get_access_token`, Stream register/event/send/upload | Retain Stream/media; port only independently useful HMAC/session semantics; reject text-only downgrade |
| Email | `email_channel.rs::imap_poll`, `email_thread_topic`, `should_skip_self_reply`, `smtp_send` | `src/email.rs::fetch_messages_blocking`, `smtp_send_blocking`, consent/auto-reply/attachments | Adapt headers/thread/dedup; retain DIVA consent/TLS/multipart |
| QQ | `qq_bot_channel.rs::get_access_token`, `fetch_gateway_url`, group/C2C send, parse events, WS state machine | `src/qq.rs::get_token`, gateway, identify/resume, `handle_event`, C2C send, group rejection | Port group/C2C/resume; verify intents and media before implementation; no fake success |

### Attribution checklist for implementation commits

1. Quote the fixed Octos SHA and exact symbol in the commit body or adjacent report.
2. State whether the code is protocol reimplementation, transformed algorithm, or retained DIVA code.
3. Link one fixture/test proving DIVA-specific bounded admission, security, receipt, and cancellation.
4. Do not copy Octos `ChannelManager`, bus, default-success optional methods, or dependencies.
