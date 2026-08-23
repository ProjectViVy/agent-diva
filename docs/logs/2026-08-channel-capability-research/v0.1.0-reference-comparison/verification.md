# 频道能力参考实现对照：验证记录

## 检查范围

本次只做本地源码和快照的静态调研，没有运行外部频道连接，也没有修改生产代码。

## 已核对

- Octos `octos-bus/src/channel.rs`：Channel 合同、bounded bus、分片、流式 finalize、
  reaction、health 和 ChannelManager 生命周期。
- Octos `qq_bot_channel.rs`：C2C/群 @、message ID、dedup、Resume/reconnect，以及出站媒体
  明确跳过的路径。
- Octos `dingtalk_channel.rs`：Webhook、签名、sessionWebhook 缓存、文本分片和媒体不支持。
- Octos `feishu_channel.rs`：WS/Webhook、签名/加密、媒体、reply endpoint、message ID、
  edit/delete 和 mock server 测试。
- ZeroClaw `zeroclaw-api/src/channel.rs`、频道共享运行时和 QQ/DingTalk/Feishu 适配器。
- OpenFang `openfang-channels/src/` 的内容块、Bridge、生命周期反应、限速/并发和频道覆盖。
- 当前 agent-diva `agent-diva-channels` 与 `agent-diva-core/src/bus` 的消息合同和 QQ/
  DingTalk/Feishu 当前能力。

## 结果

静态证据支持研究包中的结论：Octos 更贴近 agent-diva 的分层和 Channel/Bus 形状，但
ZeroClaw 在 QQ 覆盖面、频道共享可靠性和高级交互方面更适合作为验收基线；Octos 的
DingTalk 文本 Webhook 不应替代 agent-diva 当前 Stream 主路径。

## 未执行

- 未运行 `just fmt-check && just check && just test`，因为本次无 Rust 代码变更。
- 未连接 QQ、DingTalk 或 Feishu 真实服务；真实通道 E2E 应在正式 Epic 施工阶段单独验证。
