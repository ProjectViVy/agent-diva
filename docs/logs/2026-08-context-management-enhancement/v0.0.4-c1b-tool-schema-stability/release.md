# C1b Release

## 发布方式

- 作为本地 `agent-diva-pro` 分支的单一 focused commit 交付。
- 不 push、不部署；由后续集成流程决定远端发布。

## 兼容性

- `ToolRegistry::register()` 签名和默认 CORE 语义保持不变。
- 新增的 `ToolSchemaPartition` 与显式分区注册接口是向后兼容扩展。
- `get_definitions()` 的数组顺序有意从 HashMap 遍历序改为稳定的分区/名称序；工具调用
  名称、schema 内容和执行路径不变。

## 回滚

- 回滚本迭代 commit 即可恢复原注册表输出；无配置、数据库或持久化迁移。
