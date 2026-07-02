# agent-diva-nano：边界与文献索引

**nano** 已从主 workspace **拆出**，源码在 **`external/agent-diva-nano/`**，由 **`external/Cargo.toml`** 单独构建。状态与操作规则见 [nano-externalization-status.md](./nano-externalization-status.md)；迁出独立 git 仓库见 [agent-diva-nano-extracted.md](./agent-diva-nano-extracted.md)。

**产品语义**见 [nano-decoupling-preparation-plan.md](./nano-decoupling-preparation-plan.md)。

---

## 当前事实（以仓库为准）

| 项 | 说明 |
|----|------|
| 主 workspace | **不含** `agent-diva-nano`（见根 [Cargo.toml](../../../../Cargo.toml)） |
| **`agent-diva-cli`** | **仅** 依赖 **`agent-diva-manager`**；**无** `nano` feature |
| **nano 构建** | `cd external && cargo build -p agent-diva-nano` |
| **nano 依赖** | 通过 `path = "../../agent-diva-*"` 指向主仓核心 crate；迁出后改为 crates.io/git |

---

## 安全约束

- **禁止** 为「加速」而删除或掏空 `agent-diva-manager/src` 实质实现（无备份/无分支）。
- **禁止** 未经评审将 `agent-diva-nano` **重新加入**根 workspace 或恢复 CLI 对 nano 的 path 依赖（若产品变更须同步更新 [nano-externalization-status.md](./nano-externalization-status.md)）。

---

## 文献索引

- [agent-diva-nano-implementation-plan.md](./agent-diva-nano-implementation-plan.md) — 路由与契约对照（nano 与 manager 应对齐对外 API）
- [agent-diva-nano-architecture.md](./agent-diva-nano-architecture.md) — 网关拓扑与模块地图
- [minimal-gui-agent-diva-implementation-plan.md](./minimal-gui-agent-diva-implementation-plan.md)
- [crates-io-publish-strategy.md](./crates-io-publish-strategy.md)
- [docs/packaging.md](../../../packaging.md)

### 归档文献（低频背景调研）

- [archive/research/standalone-bundle-research.md](../research/standalone-bundle-research.md) — 单体安装包技术路线调研
- [archive/research/windows-standalone-app-solution.md](../research/windows-standalone-app-solution.md) — Windows 独立 App 与网关服务化方案
- [archive/architecture-reports/](../architecture-reports/) — OpenClaw / Zeroclaw 对照与 SOUL 深度分析（见该目录 README）

---

## 修订记录

| 日期 | 摘要 |
|------|------|
| 2026-03-22 | 与代码同步：CLI 曾讨论 full/nano feature（已移除） |
| 2026-03-22 | 长篇背景调研迁至 [`docs/dev/archive/`](../../README.md)（见「归档文献」） |
| 2026-03-23 | **nano 迁至 `external/`**，主 CLI 仅 manager；本文压缩为索引 |
