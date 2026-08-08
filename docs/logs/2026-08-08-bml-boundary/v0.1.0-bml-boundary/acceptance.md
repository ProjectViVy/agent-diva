# Acceptance — BML 逻辑层边界实施 v0.1.0

> 判定维度：结构边界是否真实存在、是否被守卫持续执行、
> 是否破坏任何既有行为。用户视角无感知变化（纯结构重构）。

## 验收清单

- [x] BML 公开面有结构化命名空间 `agent-diva-laputa::bml`
      （存储核心 + 记录适配层，`src/bml/mod.rs`）
- [x] 边界规则有文档声明（bml/mod.rs 模块 doc + AGENTS.md 三层模型段落）
- [x] 治理层不得直写 BML 写接口由负向守卫执行
      （`tests/bml_boundary_guard.rs`，首跑零违规）
- [x] 守卫纳入持续门禁（`just bml-boundary-check` → `ci` +
      `e7-automated-release-gate`）
- [x] 既有外部引用全部兼容（workspace `cargo check` 通过）
- [x] 既有守卫行为不变（authority_boundary_guard 仅增加 pub 构造器）
- [x] 文档同步：AGENTS.md / justfile / TODOLIST.md / 本迭代四件套

## 明确非目标（本迭代不做）

- 不移动任何源文件（`pub mod typed_store` 保留）
- 不改变任何 pub API 签名与行为
- 不注册 `put_governed` seam（仍待 GMH-24 write cutover）
- 不执行未来升 A（独立 `agent-diva-bml` crate）

## 验收证据

见 `verification.md` 与提交记录。
