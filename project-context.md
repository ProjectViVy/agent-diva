---
project_name: agent-diva
user_name: Com01
date: '2026-03-30'
sections_completed:
  - technology_stack
  - language_rules
  - framework_rules
  - testing_rules
  - quality_rules
  - workflow_rules
  - anti_patterns
status: complete
optimized_for_llm: true
---

# 面向 AI 代理的项目上下文（Agent Diva）

_本文件列出实现代码时必须遵守的规则与模式，侧重容易被忽略的约定。_

---

## 技术栈与版本

| 层级 | 技术 | 说明 |
|------|------|------|
| 语言 / 工具链 | Rust **2021** edition，工作区 `rust-version = "1.80.0"` | 以根 `Cargo.toml` 为准；`clippy.toml` 的 `msrv` 与之对齐 |
| 异步 | **Tokio**（workspace 统一版本）、`async-trait`、`futures` | 新异步代码默认跟工作区现有模式 |
| 序列化 | **serde** / **serde_json** / **serde_yaml** | 配置与消息边界优先 serde |
| 错误 | **anyhow**（边界/应用层）、**thiserror**（库内错误类型） | 不要在公开 API 里随意 `unwrap`/`expect` 吞掉可恢复错误 |
| 可观测 | **tracing** + **tracing-subscriber**（含 env-filter） | 新功能用结构化日志，避免散落 `println!` |
| HTTP / 实时 | **reqwest**、**tokio-tungstenite**、`http` | 与 channels/providers 现有用法保持一致 |
| CLI | **clap** derive | 子命令与 flag 与 `agent-diva-cli` 风格一致 |
| 桌面 GUI | **Tauri 2**（`agent-diva-gui/src-tauri`） | 前端：**Vue 3**、**Vite 6**、**TypeScript ~5.6**、**Tailwind 3**、`@tauri-apps/api` ^2 |
| 任务运行器 | **just**（可选） | Windows 用 PowerShell recipe，CI/Linux 用 bash（见根 `justfile` 注释） |

**工作区 crate 边界（新增代码放对地方）：**

- `agent-diva-core` — 配置、会话/记忆、cron、心跳、事件总线等共享内核  
- `agent-diva-agent` — Agent 循环、上下文拼装、skill/子代理流  
- `agent-diva-providers` — LLM/转写 Provider 抽象与实现  
- `agent-diva-channels` — 各聊天平台适配  
- `agent-diva-tools` — 内置工具（文件、shell、web、cron、spawn 等）  
- `agent-diva-cli` — 用户可见 CLI 入口  
- `agent-diva-service` / `agent-diva-manager` — 服务与管理 API  
- `agent-diva-neuron` — 与「神经元」相关能力（按现有模块扩展）  
- `agent-diva-migration` — 版本迁移工具  
- `agent-diva-gui` — Tauri + 前端资源  

依赖：优先在**根 `Cargo.toml` 的 `[workspace.dependencies]`** 声明版本，各 crate 用 `{ workspace = true }` 引用；crate 间用 `path = "..."`，勿随意复制粘贴版本号。

### 模块级上下文（子智能体生成）

各 crate 目录下另有精简 `project-context.md`，改具体包时优先阅读对应文件：

| 路径 | 说明 |
|------|------|
| `agent-diva-core/project-context.md` | 配置、总线、会话/记忆、日志、错误类型 |
| `agent-diva-neuron/project-context.md` | 神经元契约与 LLM 调用边界 |
| `agent-diva-agent/project-context.md` | Agent 循环、上下文与 skill 流 |
| `agent-diva-providers/project-context.md` | LLM / 转写 Provider trait 与实现 |
| `agent-diva-channels/project-context.md` | 各聊天平台适配与网络约定 |
| `agent-diva-tools/project-context.md` | 内置工具与安全边界 |
| `agent-diva-cli/project-context.md` | CLI 子命令与入口 |
| `agent-diva-service/project-context.md` | 服务进程与集成 |
| `agent-diva-manager/project-context.md` | 管理 API |
| `agent-diva-migration/project-context.md` | 版本迁移工具 |
| `agent-diva-gui/project-context.md` | Vue 前端 + Tauri 2 桌面壳 |

---

## 关键实现规则

### 语言相关（Rust）

- 使用 **Edition 2021**；格式化以根目录 **`rustfmt.toml`** 为准：`max_width = 100`、`reorder_imports = true`、4 空格缩进。  
- **`cargo clippy --all -- -D warnings`** 为事实上的质量标准（见 `justfile` 的 `check`）；Clippy 额外约束见 **`clippy.toml`**（如 `cognitive-complexity-threshold = 25`）。  
- 异步：遵循现有 Tokio 运行时用法；跨 crate 的 trait 对象注意 `Send`/`Sync` 与 `'static` 边界。  
- 配置与用户数据路径：遵循 **`dirs`** / 项目现有配置加载方式（`config` crate、`dotenv` 等），不要硬编码绝对路径。  
- **API 密钥与令牌**：仅通过配置文件或环境变量注入；禁止写入源码、禁止在日志中打印完整密钥。

### 框架相关

- **Gateway** 是会话、路由与 channel 连接的中心；消息经总线进出 Agent 循环与 LLM/Tools。改消息流或会话生命周期时先读现有 `core`/`agent` 实现。  
- **Skills**：能力通过 Markdown 等形式扩展；新增 skill 文档与加载约定需与现有 agent 侧解析逻辑一致。  
- **Tauri / 前端**：命令与事件命名与现有 `src-tauri` 桥接保持一致；前端构建需通过 `vue-tsc --noEmit && vite build`（见 `package.json` scripts）。  
- **Provider / Channel**：新实现应实现已有 trait，并注册到与现有一致的工厂或配置 schema，避免破坏 `config.json` 结构。

### 测试

- 工作区标准：`**cargo test --all**`（`just test`）。  
- HTTP/外部依赖：优先使用 **`mockito`** / **`wiremock`** 等已有测试依赖，避免测试命中真实网络。  
- 临时目录用 **`tempfile`**；异步测试可用 **`tokio-test`**。  
- 集成测试与单元测试的划分遵循各 crate 现有 `tests/` 与模块内 `#[cfg(test)]` 布局。

### 代码质量与风格

- 提交前理想顺序：`just ci` → `fmt-check` + `clippy -D warnings` + `test`。  
- `public` API 变更时考虑对 migration、GUI、CLI 的连带影响。  
- 导入顺序交给 `rustfmt`；不要为「美观」关闭 `reorder_imports`。  
- 复杂逻辑：在符合 Clippy 复杂度阈值前提下，优先小函数与清晰错误上下文（`anyhow::Context` 等），而非巨型 match。

### 开发工作流

- 本地：**`just`** 为推荐入口（`build`、`check`、`test`、`run` 等）。  
- Windows 与 Unix 的 shell 差异已在 `justfile` 中处理；新增 recipe 时需考虑是否在 CI（bash）上可运行。  
- 发布相关脚本在 `scripts/`（含各平台打包）；改安装路径或 bundle 结构时同步文档（如 `docs/packaging.md`）。

### 切勿遗漏（反模式与安全）

- 不要在生产路径使用 `unwrap()`/`expect()` 处理用户输入、网络响应或文件 IO。  
- 工具执行（shell、文件、spawn）：必须尊重现有权限与安全边界，不扩大默认攻击面。  
- 新增 channel/provider 时：超时、重试、退避与取消（drop/cancel token）需与现网逻辑一致。  
- 勿在无关 crate 中引入 GUI 专用依赖；保持 `agent-diva-gui` 与核心库的依赖方向清晰。  
- README 写「Rust 1.70+」与 workspace `rust-version` 可能不一致时，**以 `Cargo.toml` / `clippy.toml` 的 MSRV 为准**。

---

## 使用说明

**对 AI 代理：**

- 在实现或修改本仓库代码前阅读**本文件**；若改动集中在某一 crate，再读该目录下的 **`project-context.md`**。  
- 严格遵守上述规则；不确定时采用更保守、更安全的一方。  
- 技术栈或架构变更后应更新本文件及受影响模块的 `project-context.md`。

**对维护者：**

- 保持正文精简，只保留对代理「非显而易见」的约束。  
- 依赖或 MSRV 变更时同步「技术栈」表与 `rust-version`/`clippy.toml`。  
- 可定期删除已变成常识的条目。

最后更新：2026-03-30
