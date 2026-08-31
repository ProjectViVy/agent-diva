# C5-I Gate 1 共享 Adapter/Services seam 与 TCK

## Scope

本迭代在隔离 `feat/channel-epic` worktree 中完成 C5-I 第一阶段：冻结原生
`ChannelAdapter` 的共享 services seam、附件内容寻址合同、权限匹配辅助和构造工厂
失败语义。六个外部平台 adapter、Manager 生产装配和 C6 clean break 不在本迭代。

## Deliverables

- `AdapterServices` 与 `ChannelAttachmentStore` typed seam。
- `IngressAttachment`、`StoredAttachment`、`AttachmentStoreError` 及 SHA-256/path/MIME/size
  校验。
- `external_message_envelope`、receipt builders、`execution_error` 和空列表 allow-all
  的 `is_sender_allowed` helper。
- `build_active_adapters` 的显式 unavailable 语义：启用但尚未落地的 native adapter 不会
  静默返回空集合或默认成功 no-op。
- `src/adapters/mod.rs` 原生 adapter 扩展点和共享 TCK/fixture 说明。
- 修正频道 crate `AGENTS.md` 的 allowlist 与 C5/C6 ownership 说明。

## Handoff truth

共享契约已可供六个频道 agent 使用。Gate 2 仍必须逐频道补齐 Octos wire 行为、真实
capability evidence、fixture/mock 和 receipt；QQ intents/media 继续保持 Blocked。

实现提交：`87e7ff20`；文档修正提交：`da7e0410`。
