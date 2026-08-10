# C1a Release

## 发布方式

- 作为本地 `agent-diva-pro` 分支的单一 focused commit 交付。
- 不 push、不部署；由后续集成流程决定远端发布。

## 兼容性

- `LLMProvider` 新方法提供默认实现，第三方 provider 实现无需立即修改。
- 默认 transport 为保守的 `UserContextEnvelope`，避免兼容端点错误处理或提升中途
  system message。
- `NativeContextBlock` 尚无通用 wire 表达，选择该能力会返回明确错误而非降级猜测。

## 回滚

- 回滚本迭代 commit 即可恢复 C1-0 之前的生产 wire shape；不涉及数据迁移。
