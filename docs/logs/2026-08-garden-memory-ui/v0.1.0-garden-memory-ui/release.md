# Garden 记忆工作台批次 · Release

## 发布方式

本批次为本地开发迭代交付，不涉及版本发布或外部部署：

- 后端新端点随 `agent-diva-manager`（本地 127.0.0.1 gateway）发布，无独立发布物
- GUI 随 `agent-diva-gui`（Tauri 桌面应用）下一次常规打包发布
- 无数据迁移：`memory.sqlite3` schema 不变，新增能力为只读查询 + 既有 proposal 机制复用
- 无配置变更：无需新增配置文件或环境变量

## 灰度/回滚

- 端点与 UI 均为增量：`bml_routes` 挂载于 build_router，删除端点复用既有 `memory_remove` proposal 路径（治理层无直写），不改变既有行为
- 回滚：回退对应提交即可；`LaputaError::MemoryStore` 变体为纯增量，不影响既有错误码
