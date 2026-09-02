# Acceptance

产品/架构负责人验收以下事实：

- Telegram、Discord、Feishu、DingTalk、Email、QQ 各有一份独立端点级扫描报告。
- 报告能指出 Octos API/function、DIVA 当前实现、目标行为和 Port/Adapt/Retain-DIVA/Reject 决策。
- 图片识别链路、群聊、审批和权限不再停留在泛化描述，而是有 owner、依赖、fixture 和阻塞状态。
- DingTalk 明确保留 DIVA Stream/media，Email 明确保留 consent/TLS/multipart，Feishu 明确保留 WS/reaction，QQ 明确群/media 未完成。
- 每个 target-true capability 都有 evidence manifest 条目，未验证能力仍是 planned/blocked。
- 文档没有把 C5-P2 描述成产品实现完成，C5-I、C5-V 和 C6 仍然开放。
- 所有验证命令和 lock/commit/handoff 信息已写入 `verification.md` 与 `release.md`。
