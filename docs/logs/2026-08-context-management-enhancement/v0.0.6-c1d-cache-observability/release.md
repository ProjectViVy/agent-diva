# C1d Release

## 发布方式

- 作为本地 `agent-diva-pro` 分支的单一 focused commit 交付。
- 不 push、不部署；由后续集成流程决定远端发布。

## 兼容性

- `LLMProvider::prompt_cache_profile` 是带安全默认值的新增方法，现有 provider/mock 无需迁移。
- `ToolRegistry::get_definitions()` 保持不变；新增 `get_definition_set()` 暴露 CORE 边界。
- `TokenLedgerEntry` 仅新增 serde-default 可选字段，旧 JSONL 行继续反序列化。
- 无配置、数据库或 Memory authority 迁移；provider raw model ID 规则不变。

## 回滚

- 回滚本迭代 commit 即可恢复旧 cache-control 和 usage 形态；不会修改既有 Memory 数据。
