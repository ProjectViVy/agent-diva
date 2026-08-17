# agent-diva-channels

## OVERVIEW

Chat platform adapters for Agent Diva. Each platform is a handler module behind the `ChannelHandler` trait, coordinated by `ChannelManager`.

## STRUCTURE

Flat module layout under `src/`: one file per channel plus `common/` for shared helpers.

## WHERE TO LOOK

| File / Directory | Purpose |
|---|---|
| `src/lib.rs` | Module declarations and public re-exports. |
| `src/base.rs` | `ChannelHandler` trait, `BaseChannel`, `ChannelError`, `ChannelHandlerPtr`. |
| `src/manager.rs` | `ChannelManager`: validation, initialization, start/stop, runtime updates. |
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

- Add a new channel by creating `src/<channel>.rs` and exporting it from `src/lib.rs`.
- Name the concrete type `{Platform}Handler` and implement `ChannelHandler`.
- Use `BaseChannel` for allow-list checks and inbound message routing.
- Register required fields in `manager.rs` `channel_validation` and `build_updated_handler`.
- Keep platform SDK imports and transport code inside the channel module.
- Re-export the handler in `src/lib.rs`.
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

- `BaseChannel::new` defaults to `deny_by_default = true`; an empty `allow_from` denies all senders.
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
