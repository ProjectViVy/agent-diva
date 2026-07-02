# 极简 GUI Agent Diva：实施计划

> **`agent-diva-nano`** 已迁至 **`external/agent-diva-nano/`**（嵌套 workspace），**主 CLI** 不再带 `nano` feature。状态见 [nano-externalization-status.md](./nano-externalization-status.md)；安全与索引见 [agent-diva-nano-master-spec.md](./agent-diva-nano-master-spec.md)。**产品语义**与 [nano-decoupling-preparation-plan.md](./nano-decoupling-preparation-plan.md) 对齐：`agent-diva-cli` 为正式产品；nano 为 **模板线 / 后续独立 starter**，**不作为第二官方 SKU**。本文保留 **方案比选与分阶段讨论**；**以当前 `Cargo.toml` 与源码为准**。

本文档在 [crates-io-publish-strategy.md](./crates-io-publish-strategy.md) 第 11–14 节调研结论基础上，给出 **设想中的** 分阶段计划与方案比选。实施时需以当时仓库代码为准迭代本清单。

> **能力约定（最简 / 模板向路径）**：**不包含** 需独立下载的 **Desktop companion**（不构建、不依赖 [`agent-diva-gui`](../../../../agent-diva-gui/src-tauri/Cargo.toml)；**companion 不属于 crates.io 发布闭包**）；**包含** 终端 **TUI**（`agent-diva tui`）以及 CLI 其余子命令。文件名中的「GUI」反映文档起源与正式线对照；**最简主路径 = CLI + TUI + 可选 `gateway run`**；桌面 Tauri 应用属 **独立分发轨**。

---

## 1. 文档关系

| 文档 | 作用 |
|------|------|
| [crates-io-publish-strategy.md](./crates-io-publish-strategy.md) | crates.io 与 GUI 分发双轨、极简变体是否需要结构性改造的结论 |
| [nano-decoupling-preparation-plan.md](./nano-decoupling-preparation-plan.md) | 解耦准备：正式线 / 模板线、发布语义、迁移顺序与阶段边界 |
| [agent-diva-nano-implementation-plan.md](./agent-diva-nano-implementation-plan.md) | **官方最简实现（模板线前身）**：`agent-diva-nano` crate、API 契约与分阶段交付 |
| [agent-diva-nano-architecture.md](./agent-diva-nano-architecture.md) | nano：**网关进程、Desktop companion 子进程、TUI/CLI 路径、Manager/HTTP/bus** 与迁移代码地图 |
| [docs/packaging.md](../../../packaging.md) | 安装包与 CI 构建（NSIS/MSI、deb、DMG 等） |
| [standalone-bundle-research.md](../research/standalone-bundle-research.md) | 单体包、守护进程与控制面板等背景调研（可选阅读） |

---

## 2. 目标与非目标

### 2.1 目标（最简 / 模板向讨论；与正式线分层）

- **默认正式路径**：面向开发者与 headless 用户的 **`cargo install agent-diva-cli`**（二进制 `agent-diva`）及 **默认依赖 `agent-diva-manager`** 的叙事，见 [crates-io-publish-strategy.md](./crates-io-publish-strategy.md) 与 [nano-decoupling-preparation-plan.md](./nano-decoupling-preparation-plan.md)；**不与「nano = 第二官方产品线」混写**。
- **主入口（最简路径）**：**终端** —— **`tui` 子命令**（ratatui）与 **`chat` / `agent` / `gateway run`** 等 CLI 能力；**不**将独立下载的 **Desktop companion**（Tauri）纳入该路径的构建闭包。
- **依赖最小化（讨论向）**：构建产物 **可不包含** `agent-diva-manager` 与 **`agent-diva-gui`**（及 companion 专属传递依赖，如 `agent-diva-neuron` 是否保留以 `Cargo.toml` 为准）；减少与「独立 HTTP 管理服务器」心智绑定的默认路径。解耦时**允许**调整边界，但**不以继续增加 crate 数量为目标**，优先在**更少 crate**内收敛。
- **可打包发布**：正式 CLI 以 **crates.io + 轻量安装包** 为主；**Desktop companion** 以 **Release / 安装包** 为主（参见 [docs/packaging.md](../../../packaging.md)），**不属于 crates.io 主闭包**。
- **与 nanobot 取向对齐（产品层）**：偏 **单盘、轻量、个人助手**；技术实现仍属 Agent Diva Rust workspace（见 [crates-io-publish-strategy.md 第 12 节](./crates-io-publish-strategy.md)）。

### 2.2 非目标（可作为后续阶段）

- **最简路径默认不包含**：**Desktop companion** 安装体验与 `agent-diva-gui` 构建产物（与「带 TUI、不带 companion」一致）。
- 默认不承诺：与当前正式线（**CLI + manager + companion 可选**）**完全同一套** 远程 manager API、全频道矩阵、全工具集 **同时** 在最简路径中开箱即用。
- 不在本计划中规定：具体商号、安装包签名主体、应用商店账号（仅提示与 packaging 衔接）。
- 不在本文中锁定 crate 最终命名（实施时与 crates.io 可用名一致即可）。

---

## 3. 现状差距

### 3.1 依赖链

```
agent-diva-gui → agent-diva-cli → agent-diva-manager（硬依赖）
```

见 [agent-diva-gui/src-tauri/Cargo.toml](../../../../agent-diva-gui/src-tauri/Cargo.toml)、[agent-diva-cli/Cargo.toml](../../../../agent-diva-cli/Cargo.toml)。

### 3.2 运行时耦合

`agent-diva-cli` 中网关路径将 **agent 运行时** 与 **`agent-diva-manager` 的 `Manager` / `run_server`（HTTP 等）** 编排在一起。去掉 manager 不等于删除未使用模块，而是要 **迁移「编排职责」** 到新的落点（如 feature 后代码路径，或既有 crate 内部的收敛模块），**不默认导向新增独立 runtime crate**。

### 3.3 与 standalone 调研的关系

[standalone-bundle-research.md](../research/standalone-bundle-research.md) 强调常驻服务、控制面板与守护进程等需求；**Desktop companion** 可能选择 **单进程内嵌网关** 以减少「companion + 子进程」模型；**最简路径** 则天然偏 **CLI/TUI ± 独立 `gateway run` 进程**。实施时需在计划中写明：**各分发路径**的进程与 HTTP 边界。

---

## 4. 方案比选

### 方案 A：`agent-diva-cli` 使用 Cargo feature 切分 manager

| 优点 | 缺点 |
|------|------|
| 改动相对集中；正式线与最简路径 **同仓库** 共存 | `main.rs` / 网关路径 **条件编译** 复杂，需严防 feature 泄漏 |
| 用户仍可能 `cargo install` 单一 crate（若未来上架） | docs.rs 与默认 feature 策略需文档化 |

**要点**：`default` feature 保持现有行为；`minimal`（名称可调整）关闭对 `agent-diva-manager` 的依赖，并提供等价 **内嵌 orchestration**（优先从既有 crate 或新模块中收敛实现）。

### 方案 B：在较少 crate 边界内收敛运行时职责

| 优点 | 缺点 |
|------|------|
| 边界清晰；CLI / companion **共用** 同一套「启动与生命周期」API | **可能**增加可发布单元与 semver 维护面；与「少 crate」目标需权衡 |
| 便于单元测试 orchestration，无需跑完整 CLI | 需一次 **从 `main.rs` 抽逻辑** 的重构窗口 |

**要点**：`agent-diva-cli` 的 `gateway` 与 companion 侧启动应通过**同一套可复用的编排面**（具体是库边界还是模块边界以实施为准）对齐；`agent-diva-manager` 仅在正式路径或独立二进制中引用（若仍保留独立 manager 模式）。**`agent-diva-nano` 当前是官方最简实现、后续迁出为 starter/template**，**不作为**「必须新增的长期运行时宿主 crate」的默认结论；优先在**较少 crate**内收敛职责。

### 方案 C：companion 直接依赖 `agent-diva-agent` / `agent-diva-core` 等，绕过 `agent-diva-cli`

| 优点 | 缺点 |
|------|------|
| companion 依赖图最直接 | **极易重复** 网关启动逻辑，与 CLI 漂移 |
| 短期可能看似行数少 | 长期维护成本高，**一般不推荐** 作为主方案 |

### 4.1 推荐路径

**首选（讨论向）：方案 A + 在既有 crate 边界内收敛职责**

- 用 **既有 crate 内的模块** 收敛「无 manager 时的网关等价行为」，API 稳定后 companion 与 CLI 共用；**不以继续新增 crate 为目标**。
- 在 **`agent-diva-cli` 上用 feature** 控制是否链接 `agent-diva-manager`，以便 **正式路径** 行为与现网一致，**最简路径** 构建不拉取 manager。

> **实施说明**：编排职责当前仅作为**解耦讨论中的落点**，最终放在哪个既有 crate / 模块中，以实施后的代码与 `Cargo.toml` 为准；**当前文档阶段不预设新增独立运行时 crate**。职责边界、HTTP 路由验收清单与阶段划分见 [agent-diva-nano-implementation-plan.md](./agent-diva-nano-implementation-plan.md)。

**备选**：可 **仅方案 A** 在 `agent-diva-cli` 内用模块 + feature 拆分；是否再抽出独立库以 **耦合度与 crate 数量权衡** 为准，**不以「必须新增 crate」为路线图承诺**。

---

## 5. 分阶段里程碑

### 阶段边界与迁移顺序

- **当前阶段**（与 [nano-decoupling-preparation-plan.md](./nano-decoupling-preparation-plan.md) 一致）：**文档同步、边界确认、耦合盘点**；**不**在本轮落地具体代码或迁仓。
- **后续顺序**：**解耦准备 → 主线解耦 / 正式线收口 → 将 `agent-diva-nano` 迁出 workspace（独立 starter/template）**。下列 Phase 0–4 为**获准后的工程任务清单**，非当前默认排期。

### Phase 0：需求冻结与范围签字

- [ ] 明确最简路径 **默认开启的能力**：例如 TUI + 本地 `gateway` + 单一 provider；channels 是否默认全关；**不含 Desktop companion 构建** 已锁定，无需再议是否「仅 companion 聊天」。
- [ ] 明确 **是否保留** `--remote` / 连接外部网关 HTTP 的 story；若保留，本地 `gateway run` 是否仍起 axum（供 remote 自连或脚本）。
- [ ] 与产品/支持约定：**安装包名称、渠道** —— 正式 CLI（`cargo install` / headless 包）与 **独立下载的 Desktop companion** 区分，避免用户混淆。

### Phase 1：依赖图与 feature 设计

- [ ] 画出 **模板线（`external/agent-diva-nano`）vs 主 CLI（manager）** 两套依赖 DAG（crate 级），更新本文「第 6 节 crates.io 闭包」草稿表。
- [ ] 在设计上消除 **companion → 全量 CLI → manager** 的硬传递（最简路径 **不构建 companion**，无此边）；companion 可演进为依赖 **`agent-diva-nano`** 等；**当前**主 CLI **固定**依赖 manager，若未来引入 **minimal** 类 feature 再单独评审。
- [ ] 评估 **companion（Tauri）** 对 HTTP/SSE 的假设（最简路径 **无 companion 构建**，该项仅作用于 companion 构建）；最简路径若仍起网关 HTTP，明确与 `--remote`/自动化的契约。

### Phase 2：运行时拆分与接口稳定

- [ ] 从当前网关路径抽出 **生命周期**：MessageBus、AgentLoop、cron、与 **companion** 的桥接（含 cron→`api`/SSE 等；最简路径无 companion 时是否仍写入 `api` 通道由产品决定）。
- [ ] 实现 **无 manager** 路径下的 HTTP 服务策略：**无 HTTP** / **轻量内嵌 axum（若仍需要 SSE）**——二选一并写 ADR 简短记录。
- [ ] 保证 **正式路径（当前默认：CLI + manager）** 下行为与重构前 **可对比**（集成测试或手工清单）。

### Phase 3：打包脚本与说明

- [ ] **companion 线**：调整 `agent-diva-gui` 的 Cargo 依赖，使 companion 构建可走 **nano / 无 manager** 路径（与最简路径无冲突）。
- [ ] **最简路径**：打包目标 **不包含** `agent-diva-gui`；更新 [docs/packaging.md](../../../packaging.md) 与 `just` 等：增加 **minimal（CLI+TUI）** 构建目标或 feature（具体由实施时选定）；companion 专用脚本（如 `package-windows-gui.ps1`）标注为 **Desktop companion 专用**。
- [ ] 文档：**用户可见** 的「最简路径（无 companion、有 TUI）vs 正式 CLI + 可选 companion」说明（安装包名、功能差异）。

### Phase 4：测试与发布矩阵

- [ ] CI：`cargo build` / `cargo test`（视范围）对 **full** 与 **minimal** 各至少一条线。
- [ ] 冒烟：**最简路径**：`tui` 会话、`gateway run` + `chat`/`agent`、（若保留）cron；**正式线 + companion**：另加 companion 启动与一轮对话。
- [ ] crates.io：若极简 CLI/runtime 单独发布，更新 [crates-io-publish-strategy.md 第 5、14 节](./crates-io-publish-strategy.md) 中的 crate 列表与顺序。

---

## 6. crates.io 发布闭包（草稿表，实施后更新）

**说明**：**Desktop companion**（`agent-diva-gui` / Tauri）为 **独立下载**，**不计入** crates.io 上 `cargo install agent-diva-cli` 的发布闭包。下表第二列表示 **正式 CLI 闭包**（默认含 `agent-diva-manager` 等，与 [nano-decoupling-preparation-plan.md](./nano-decoupling-preparation-plan.md) 一致）；第三列为 **最简 / 模板向路径**（讨论向，非默认正式安装叙事）。

在实施完成并固化 `Cargo.toml` 后，将下列占位替换为 **是/否** 与说明。

| Crate | 正式 CLI 闭包（crates.io；默认 `agent-diva-cli` + manager） | 最简 / 模板向（无 companion 构建，有 TUI） |
|-------|----------------------------------------------------------|---------------------------------------------|
| agent-diva-core | 是 | 是 |
| agent-diva-providers | 是 | 是 |
| agent-diva-tools | 是 | 视工具裁剪而定 |
| agent-diva-channels | 是 | 视聊天频道裁剪而定 |
| agent-diva-agent | 是 | 是 |
| agent-diva-neuron | 是（companion 常用；属 companion 构建依赖，非 CLI tarball 必需） | **否（目标：无 companion）** |
| agent-diva-gui | **否（独立分发，非 crates.io CLI 闭包）** | **否（目标：不构建 companion）** |
| agent-diva-manager | 是 | **否（讨论目标）** |
| agent-diva-cli | 是 | **是**（含 `tui`） |
| agent-diva-nano（当前官方最简实现；后续迁出为 starter/template） | 视依赖图而定 | **是**（讨论向） |

---

## 7. 风险与回滚

| 风险 | 应对 |
|------|------|
| 双轨逻辑分叉导致 bug 仅出现在 minimal | Phase 4 强制矩阵；关键路径共享库化 |
| companion 与后端 API 契约变更 | 版本化 API 文档；**主要影响 Desktop companion**；最简路径以 TUI/本地 CLI 为主 |
| docs.rs feature 超时或构建失败 | 默认 feature 保持轻量；重依赖放 optional feature |
| 发布节奏分裂 | 同一语义化版本下对齐 tag，或在 README 标明分发路径 |

**回滚**：保留 `full`/`default` 构建路径不变；minimal 可通过 feature 关闭或 yank 单独安装包渠道，直至稳定。

---

## 8. 参考链接

- [nano-decoupling-preparation-plan.md](./nano-decoupling-preparation-plan.md)
- [crates-io-publish-strategy.md](./crates-io-publish-strategy.md)
- [agent-diva-nano-implementation-plan.md](./agent-diva-nano-implementation-plan.md)
- [agent-diva-nano-architecture.md](./agent-diva-nano-architecture.md)
- [docs/packaging.md](../../../packaging.md)
- [AGENTS.md](../../../../AGENTS.md)（`.workspace` 与姊妹项目约定）
- nanobot：<https://github.com/HKUDS/nanobot>

