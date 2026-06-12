# Nano Externalization Status

## Current State

- **`agent-diva-nano` 源码**位于 **`external/agent-diva-nano/`**，由 **`external/Cargo.toml`** 单独 workspace 构建；**不是**根 workspace 成员。
- 主 workspace **不再**在本地构建 nano（`cargo build` 于根目录不包含 nano）。
- **`agent-diva-cli`** **无** `nano` feature；本地网关 **仅** manager 路径。
- 主 CLI 与发版叙事以 **`agent-diva-cli` + `agent-diva-manager`** 为准。
- 将 nano **完全迁出** monorepo 的步骤见 [agent-diva-nano-extracted.md](./agent-diva-nano-extracted.md)。

## Operator Rules

- 在 monorepo 内验证 nano：`cd external && cargo check -p agent-diva-nano`（勿在根目录使用 `-p agent-diva-nano`）。
- **勿**将 `agent-diva-nano` 加回根 `[workspace].members`，**勿**在 `agent-diva-cli` 中恢复 path 依赖 nano（除非经显式产品决策并更新本文）。
- 历史 nano bootstrap 日志仅作记录，不代表当前目录布局。

## Install Entry

- **主产品**：`cargo install agent-diva-cli`（自根 workspace 发布）。
- **Nano 线**：在迁出前于 monorepo 内 `cd external && cargo publish -p agent-diva-nano`（若配置允许）；迁出后于 **独立仓库** 发布，见 [agent-diva-nano-extracted.md](./agent-diva-nano-extracted.md)。
