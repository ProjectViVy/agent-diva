# C5-I Gate 2：六频道原生适配器

日期：2026-09-01
隔离分支：`feat/channel-epic`
范围：Telegram、Discord、Feishu、DingTalk、Email、QQ 的 native `ChannelAdapter`，共享
factory/TCK 接线与 Octos 对照证据。

## 完成内容

- 六个频道均有独立 `agent-diva-channels/src/adapters/*.rs` 原生实现，不调用 legacy
  `ChannelHandler`，不引入 Octos runtime。
- 统一使用 `AdapterContext`/Fabric admission、`AdapterServices` typed attachment seam、
  `ChannelEnvelopeV1`、allowlist、dedup、health、typed receipt 和 unsupported error。
- Telegram/Discord/Feishu/DingTalk/Email 保留各自已存在的强 DIVA transport；QQ 接入官方
  token/Gateway、C2C/group text、`msg_seq`、回复 ID、heartbeat/resume/reconnect。
- `build_active_adapters` 现在为六个启用频道构造真实 adapter，但仍不负责注册、监督或启动，
  Manager C6 cutover 保持关闭。
- 每频道 fixture README/JSON、Gate2 implementation note、证据 manifest 和 TODO 交接均已补齐。

## 明确未关闭项

`evidence-manifest.md` 的 Gate2 行仍为 `implemented/partial`，因为本阶段没有凭据或真实外部
网络。QQ D-013（Identify intents）与 D-014（官方媒体上传）继续 blocked/unsupported；C5-V
仍需本地 HTTP/WS/IMAP/SMTP transcript、全量 capability TCK、QQ 真机纵向 smoke 后才能勾选。
