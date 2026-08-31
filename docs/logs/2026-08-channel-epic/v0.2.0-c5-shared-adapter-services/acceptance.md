# Acceptance

架构/产品负责人验收以下事实：

- native adapter 可通过 `AdapterServices` 获取唯一的 typed attachment authority，且无需
  修改 `AdapterContext` 或引入第二套 bus/runtime。
- 附件引用只能由内容 digest 证明，包含 MIME、大小、SHA-256 和安全文件名校验；共享 TCK
  能证明读回一致、路径拒绝和大小上限错误。
- 外部入站 envelope 固定为 `ExternalUser`，不会携带 Owner context；receipt 默认是
  `Accepted`，没有平台 ID 时不会伪造 ID。
- 空 `allow_from` 明确是 allow-all，非空列表支持既有 wildcard/compound ID 行为；平台
  agent 仍需增加自己的 DM/group/mention policy。
- 启用尚未实施的频道时，factory 返回 typed unavailable error，而不是空成功或 no-op。
- 当前迭代没有实现任何 Telegram/Discord/Feishu/DingTalk/Email/QQ wire 能力；也没有
  Manager 生产装配或 C6 删除。
- 共享 TCK、workspace 测试和标准格式/lint 门禁结果已记录；MSRV 与 legacy all-targets
  clippy 限制已写入 TODO，未被隐式关闭。
