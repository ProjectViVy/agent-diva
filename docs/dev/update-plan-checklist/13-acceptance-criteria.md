# 验收标准

## 功能

- [ ] `update_plan` 后继续获得最终回复，GUI 不停留在输入中。
- [ ] 清单卡不重复，且不会覆盖上一用户轮次。
- [ ] 工具名保留，但 UI 和提示词明确为 TODO/checklist。
- [ ] Plan mode、execution TODO 与仓库 `TODOLIST.md` 保持隔离。

## 兼容

- [ ] 新输出使用 snake_case，旧 PascalCase 输入和历史卡正常解析。
- [ ] 配置键和 SSE 事件名不变，无数据库迁移。

## 质量

- [ ] Rust 与 GUI 针对性测试通过。
- [ ] GUI production build 与最小 smoke 通过。
- [ ] `just fmt-check`、`just check`、`just test` 通过，或明确记录与本改动无关的阻塞。
- [x] 13 部分开发文档齐全，无新增不安全代码或秘密。
