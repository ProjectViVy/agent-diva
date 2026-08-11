# C1c Workspace Memory Epoch Release

## 发布方式

- 作为本地 `agent-diva-pro` 分支的 focused implementation commit 和独立 docs/TODOLIST
  收口 commit 交付。
- 不 push、不部署。

## 兼容性

- `MemoryProvider` 刷新接口提供默认 no-op 实现，现有 Provider/test double 无需迁移。
- 不新增配置、数据库表或迁移；HTTP apply/rollback 响应格式保持不变。
- runtime channel 不可用时只记录结构化诊断，不把已提交 BML 伪装成失败。

## 回滚

- 回滚本迭代 implementation commit 可恢复原有 startup cache 行为，不会修改已提交的
  BML authority 数据。
