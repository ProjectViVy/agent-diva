# Release — ask_user 运行时 MVP（Phase 1）

- 版本：`v0.1.0-ask-user-runtime-mvp`
- 日期：2026-08-05

## 发布方式

代码随 `agent-diva-pro` 分支常规提交发布（Conventional Commit，聚焦本次变更文件）。
无需配置迁移：`BuiltInToolsConfig.ask_user` 与 schema `ask_user` 均有
`serde(default = "default_enabled")`，旧配置缺失字段自动取默认值 true，向后兼容。

## 行为影响（用户可感知）

- 主会话默认工具表新增 `ask_user`；无交互表面注入时（CLI/manager 当前默认），
  模型调用后得到 `unavailable`，退化为文本提问——与 Phase 1 前行为一致，无劣化。
- GUI/CLI 问题卡为 Phase 2（注入 coordinator + 消费 pending/answer）。

## 回滚

移除 `ask_user` 相关 commit 或设置配置 `tools.builtin.ask_user = false` 即关闭
工具注册；运行时无状态残留（coordinator 为进程内内存，不持久化）。
