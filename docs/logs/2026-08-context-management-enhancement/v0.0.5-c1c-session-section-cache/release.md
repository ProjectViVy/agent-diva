# C1c Release

## 发布方式

- 作为本地 `agent-diva-pro` 分支的单一 focused commit 交付。
- 不 push、不部署；由后续集成流程决定远端发布。

## 兼容性

- 现有 system prompt 和 message builder 入口保持不变，稳定内容在无失效事件时字节不变。
- `MemoryProvider::system_prompt_revision` 提供默认实现，现有第三方/test provider 无需迁移。
- `PromptSection.cache_break_reason` 从自由字符串收紧为公开枚举；仓库内调用已同步迁移。
- 无配置、数据库或持久化 schema 迁移。

## 回滚

- 回滚本迭代 commit 即可恢复逐次稳定 section 装配；不会影响既有 Memory authority 数据。
