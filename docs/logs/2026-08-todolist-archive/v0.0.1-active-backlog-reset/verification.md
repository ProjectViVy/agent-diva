# Verification

- 清理前统计：1078 行，51 个顶层完成项，61 个顶层开放项。
- 完整原文件保存在
  `docs/archive/todolist/snapshot-before-2026-08-13-cleanup.md`。
- 根 `TODOLIST.md` 只保留未完成条目，不含 `- [x]`。
- 对旧 G2D+、GA-MEM-PARITY、E0–E7、GMH 完成历史和重复验收项进行归档，不再让其
  作为当前执行顺序。
- 运行 Markdown diff whitespace 检查和链接目标存在性检查。
- 未运行 Rust/GUI 构建：本次为纯文档归档，无可执行代码变化。
