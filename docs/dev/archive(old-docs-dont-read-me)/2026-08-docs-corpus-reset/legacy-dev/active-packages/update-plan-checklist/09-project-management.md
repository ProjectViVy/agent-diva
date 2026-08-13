# 开发项目管理资料

## 工作分解

- [x] 定位 GUI 卡死事件序列。
- [x] 对照 Codex 锁定 checklist 语义。
- [x] 修复后端事件顺序和 GUI 对账。
- [x] 增加 snake_case 与旧格式兼容。
- [x] 增加针对性单元和集成测试。
- [ ] 完成工作区验证、GUI smoke 与提交。

## 依赖与阻塞

实现依赖当前工作树尚未提交的 normal-chat `update_plan` 集成，因此在共享根工作树中精确加锁并逐 hunk 暂存。其他故事改动不得进入本次提交。

## 交付里程碑

M1 为事件流不再卡死；M2 为 TODO/Plan 语义隔离；M3 为 Rust、GUI 和工作区验证通过；M4 为 iteration log 与聚焦提交完成。
