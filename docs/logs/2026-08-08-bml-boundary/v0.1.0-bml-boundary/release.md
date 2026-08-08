# Release — v0.1.0-bml-boundary

## 类型

**结构重构交付**。无二进制行为变更、无配置 schema 变更、无新依赖。
纯增量：新增 BML 命名空间模块 + 负向守卫测试 + 文档/门禁同步。

## 发布内容

```text
docs/logs/2026-08-08-bml-boundary/v0.1.0-bml-boundary/
  summary.md
  verification.md
  release.md
  acceptance.md
```

涉及文件：

- 新建：`agent-diva-laputa/src/bml/mod.rs`（BML 逻辑层公开面 + 边界声明）
- 新建：`agent-diva-laputa/tests/bml_boundary_guard.rs`（防回归守卫）
- 修改：`agent-diva-laputa/src/lib.rs`（顶层存储 re-export 改经 `bml`）
- 修改：`agent-diva-laputa/tests/authority_boundary_guard.rs`
  （`ForbiddenPattern::new` + `scan_forbidden_access` 改 pub，供复用）
- 修改：`AGENTS.md`（三层模型段落追加 BML 逻辑层声明）
- 修改：`justfile`（`bml-boundary-check`，纳入 `ci` + `e7-automated-release-gate`）
- 修改：`TODOLIST.md`（S9 完成记录，升 A 条目保留）

## 兼容性

- 外部 crate（agent / manager / migration / autodream / cli / tools / gui）零改动
- `agent_diva_laputa::typed_store::*` 与 `agent_diva_laputa::TypedMemoryStore`
  等既有引用路径全部保持可用
- 既有守卫（direct_write_guard / authority_boundaries）行为不变

## 回滚

本迭代无运行时状态变更，回滚 = 撤销单次 git 提交即可（代码纯增量，
无数据文件、无 schema 迁移）。

## 部署

无需部署步骤；随常规构建产物发布。
