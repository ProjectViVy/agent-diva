# agent-diva-nano：独立项目说明

## 当前布局（与主 monorepo 的关系）

- 源码位于 **`external/agent-diva-nano/`**，由 **`external/Cargo.toml`** 单独组成 **嵌套 workspace**。
- **根** [Cargo.toml](../../../../Cargo.toml) **不包含** `agent-diva-nano`；主产品 **`agent-diva-cli`** **仅** 依赖 **`agent-diva-manager`**，**不** 依赖 nano。
- 在 monorepo 内开发 nano：`cd external && cargo build -p agent-diva-nano`。

## 迁出为完全独立 git 仓库（推荐步骤概要）

1. **新建空仓库**（例如 `agent-diva-nano` 或 `agent-diva-starter`）。
2. 将 **`external/agent-diva-nano/`** 的内容复制为**新仓库根**（或 `git subtree split` 仅该目录历史）。
3. 将 `Cargo.toml` 中的 `agent-diva-*` **path 依赖** 改为：
   - **crates.io** 上已发布的版本（`version = "…"`），或
   - **git** 依赖指向主仓库的 tag/commit。
4. 在新仓库根添加自己的 **`[workspace]`**（若仍为单包，可仅保留 `[package]`）。
5. 从主仓库 **删除** `external/agent-diva-nano`（迁出完成后），并更新本文与 [nano-externalization-status.md](./nano-externalization-status.md)。

产品语义仍见 [nano-decoupling-preparation-plan.md](./nano-decoupling-preparation-plan.md)。
