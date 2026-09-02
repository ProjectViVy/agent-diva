# DingTalk C5-I Gate 2 implementation evidence

实现文件：`agent-diva-channels/src/adapters/dingtalk.rs`，worker commit `cbb57d72`。
实现保留 DIVA Stream/OAuth、私聊/群聊、媒体上传和 ACK/reconnect/cancel；Octos 的 HMAC 与
session-webhook 只作为兼容辅助，不会把主路径降级成 text-only webhook。

## 已接入行为

- Stream token/session 建立、群 `openConversationId` 与私聊身份保留；allowlist 和群策略在
  媒体 URL 解析及 Fabric admission 前执行。
- 图片/音频/视频/文件进入 `ChannelAttachmentStore`，上传和发送均返回明确的 typed receipt；
  unsupported edit/reaction/typing 等命令在网络调用前拒绝。
- OAuth 缓存、ACK、断线重连、取消、HMAC 签名形状和 session webhook TTL helper 均保留，
  secrets/URL 不进入日志。

证据位于 `tests/fixtures/c5/dingtalk/`：`stream-callback.json`、`token-response.json`、
`group-send-response.json`、`media-upload-response.json`；模块能力、签名、allowlist/媒体顺序
和 unsupported 零副作用测试已通过。DT-01～DT-04 仍需 C5-V 的 mock Stream/HTTP transcript
和 Fabric busy/partial-upload 证明后才能改为 `verified`。
