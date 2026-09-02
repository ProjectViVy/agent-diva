# agent-diva-channels

## OVERVIEW

Chat platform adapters for Agent Diva. Legacy platform handlers remain available only until the
atomic CHANNEL-EPIC C6 cutover; new C5 work uses native `ChannelAdapter` implementations behind
the bounded Super Channel Fabric runtime.

## STRUCTURE

Legacy modules remain flat under `src/`. Native C5 adapters live under `src/adapters/`, while
`src/runtime/` owns the shared Registry, pacing, and supervisor.

## WHERE TO LOOK

| File / Directory | Purpose |
|---|---|
| `src/lib.rs` | Module declarations and public re-exports. |
| `src/adapter.rs` | Native `ChannelAdapter`, services, and typed adapter errors. |
| `src/adapters/` | C5 native Telegram/Discord/Feishu/DingTalk/Email/QQ adapters. |
| `src/base.rs` | `ChannelHandler` trait, `BaseChannel`, `ChannelError`, `ChannelHandlerPtr`. |
| `src/manager.rs` | `ChannelManager`: validation, initialization, start/stop, runtime updates. |
| `src/runtime/` | Native adapter Registry, pacing, and supervised listener lifecycle. |
| `src/common/` | Shared HTTP client and media download helpers. |
| `src/telegram.rs` | Telegram Bot API handler. |
| `src/discord.rs` | Discord gateway handler. |
| `src/slack.rs` | Slack socket-mode handler. |
| `src/email.rs` | IMAP/SMTP email handler. |
| `src/qq.rs` | QQ OpenAPI handler. |
| `src/feishu.rs` | Feishu/Lark handler. |
| `src/whatsapp.rs` | WhatsApp bridge handler. |
| `src/dingtalk.rs` | DingTalk stream-mode handler. |
| `src/matrix.rs` | Matrix client handler. |
| `src/irc.rs` | IRC handler. |
| `src/mattermost.rs` | Mattermost handler. |
| `src/nextcloud_talk.rs` | Nextcloud Talk handler. |
| `src/neuro_link.rs` | Local neuro-link pipe handler. |

## CONVENTIONS

- Legacy-only changes add `src/<channel>.rs`, name the concrete type `{Platform}Handler`, and
  implement `ChannelHandler`; this path is not used for C5 migration work.
- C5 work creates `src/adapters/<platform>.rs`, names the concrete type `{Platform}Adapter`, and
  implements `ChannelAdapter` without calling or wrapping a legacy handler.
- Native adapters receive `AdapterServices` at construction, use the shared bounded Fabric,
  Registry, pacing, supervisor, receipt, and capability contracts, and keep platform transport
  code inside their own module.
- Keep `allow_from` executable semantics consistent: an empty list allows all senders; a populated
  list restricts senders. Platform-specific DM/group/mention policy is evaluated before media
  download and Fabric admission.
- Registering native adapters in Manager and deleting legacy handlers are C6-only operations.
- Re-export public native contracts and adapters from `src/lib.rs` only through focused shared/owner
  commits.
- Retired-channel injection points in `manager.rs` (imports, validation arms,
  `configured_channel_names` entries, `build_updated_handler` arms, startup blocks)
  carry `#[cfg(feature = "channel-*")]` gates marked `RETIRED 2026-08-18`; keep the
  `_ => None` fallbacks intact so ungated builds degrade to "unknown channel".

## ANTI-PATTERNS

- Do not put platform-specific connection logic in `manager.rs` or `base.rs`.
- Do not create a channel handler without registering it in `ChannelManager`.
- Do not share mutable runtime state between channel handlers.
- Do not add channel-specific dependencies to `Cargo.toml` unless that module is the only consumer.

## NOTES

- `BaseChannel` legacy documentation may mention historical defaults, but the executable channel
  contract is authoritative: an empty `allow_from` allows all senders and a populated list filters
  them.
- `neuro-link` binding is restricted to `127.0.0.1`/`localhost` via `validate_neurolink_host`.
- QQ uses a separate `reqwest_qq` dependency because `bots.qq.com` needs native TLS on Windows.
- **Retired channels (2026-08-18, user decision)**: `slack`, `whatsapp`, `matrix`,
  `irc`, `mattermost`, `nextcloud_talk` are source-retained but **not compiled by
  default**. Each is gated behind an opt-in Cargo feature: `channel-slack`,
  `channel-whatsapp`, `channel-matrix`, `channel-irc`, `channel-mattermost`,
  `channel-nextcloud-talk` (all default-off; `channel-slack` also pulls
  `dep:slack-morphism`). Re-enable via
  `cargo build -p agent-diva-channels --features channel-slack,channel-whatsapp,...`
  or by adding the feature to `default = []` in `Cargo.toml`.
- `tests/whatsapp_bridge_integration.rs` only compiles with `channel-whatsapp`
  (`[[test]] required-features`).
