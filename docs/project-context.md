# Agent Diva Pro — Project Context

> 本文档供 AI Agent 在本项目中工作时参考，提取自实际代码库扫描。
> 最后更新: 2026-06-02

---

## 1. Naming Conventions（命名约定）

### 1.1 Rust

| 元素 | 规则 | 示例 |
|------|------|------|
| 文件名 | `snake_case.rs` | `loop_turn.rs`, `rate_limit.rs` |
| 模块名 | `snake_case` | `pub mod agent_loop;`, `pub mod mcp_sdk;` |
| 类型/结构体/枚举 | `PascalCase` | `AgentLoop`, `ProviderRegistry`, `ChannelError` |
| 函数/方法 | `snake_case` | `fetch_provider_model_catalog`, `build_router` |
| 常量/静态 | `SCREAMING_SNAKE_CASE` | `DEFAULT_GATEWAY_PORT` |
| Crate 名 | `agent-diva-*` (kebab-case) | `agent-diva-core`, `agent-diva-tools` |
| 二进制名 | `agent-diva`, `agent-diva-service`, `agent-diva-migrate` | — |
| Trait 名 | `PascalCase`，无 `I`/`T` 前缀 | `LLMProvider`, `Tool`, `BaseChannel`, `NeuronNode`, `StorageBackend` |
| 错误枚举 | `PascalCase`，以 `Error` 结尾 | `ProviderError`, `ToolError`, `ChannelError`, `FileError` |

### 1.2 Vue / TypeScript

| 元素 | 规则 | 示例 |
|------|------|------|
| `.vue` 文件名 | `PascalCase.vue` | `ChatView.vue`, `ProviderWizardModal.vue` |
| 模板中组件使用 | `PascalCase`（不用 kebab-case） | `<ChatView />`, `<McpSettings />` |
| `.ts` 文件名 | `camelCase.ts` | `desktop.ts`, `providers.ts`, `appToast.ts` |
| 工具/常量文件 | `camelCase.ts` | `localStorageAgentDiva.ts`, `channel-icons.ts` |
| Interface 名 | `PascalCase`，DTO 后缀 `Dto` | `McpServerDto`, `SkillDto`, `ConfigStatusReport` |
| 函数名 | `camelCase` | `getConfigStatus`, `clearAgentDivaLocalStorage` |

### 1.3 测试文件命名

| 位置 | 命名规则 | 示例 |
|------|----------|------|
| Rust 内联测试 | `mod tests` 在源文件底部 | `#[cfg(test)] mod tests { ... }` |
| Rust 集成测试 | `tests/<描述性名称>.rs` | `tests/qq_reconnect_integration.rs`, `tests/ollama_streaming.rs` |
| 前端 | **无测试文件** | — |

---

## 2. Code Organization（代码组织）

### 2.1 Rust Workspace 结构

```
agent-diva-pro/
├── Cargo.toml              # Workspace root, resolver v2, 版本 0.4.9
├── justfile                # 任务运行器
├── rustfmt.toml            # 格式化配置
├── clippy.toml             # Lint 配置
│
├── agent-diva-core/        # 基础层: 类型、配置、错误、会话、内存
│   └── src/
│       ├── lib.rs          # pub mod 声明 + pub use 重导出
│       ├── error.rs        # 中央 Error 枚举
│       ├── bus/            # mod.rs + events.rs + queue.rs
│       ├── config/         # mod.rs + loader.rs + validate.rs + schema.rs
│       ├── security/       # mod.rs + config.rs + error.rs + path.rs + policy.rs + rate_limit.rs
│       ├── session/        # mod.rs + manager.rs + store.rs
│       └── ...
│
├── agent-diva-agent/       # Agent 循环: 上下文构建、技能加载、子代理
│   └── src/
│       ├── agent_loop/     # mod.rs + loop_tools.rs + loop_turn.rs + loop_runtime_control.rs
│       ├── context.rs
│       ├── skills.rs
│       └── subagent.rs
│
├── agent-diva-providers/   # LLM 提供商集成
│   └── src/
│       ├── base.rs         # LLMProvider trait + ProviderError
│       ├── registry.rs     # ProviderRegistry
│       ├── litellm.rs
│       ├── ollama.rs
│       └── http_util.rs    # 私有模块 (mod 不加 pub)
│
├── agent-diva-channels/    # 聊天平台集成
│   └── src/
│       ├── base.rs         # BaseChannel trait + ChannelError
│       ├── manager.rs      # ChannelManager
│       ├── telegram.rs, discord.rs, feishu.rs, qq.rs, ...
│       └── tests/          # 集成测试
│
├── agent-diva-tools/       # 工具系统
│   └── src/
│       ├── base.rs         # Tool trait + ToolError
│       ├── registry.rs     # ToolRegistry
│       ├── filesystem.rs, shell.rs, web.rs, ...
│       └── mcp_sdk.rs      # MCP 集成 + McpError
│
├── agent-diva-files/       # 文件管理 (内容寻址存储)
│   └── src/
│       ├── backend.rs      # StorageBackend trait
│       ├── manager.rs      # FileManager
│       └── ...
│
├── agent-diva-manager/     # HTTP 网关服务器
│   └── src/
│       ├── lib.rs          # pub use 重导出 Manager, run_local_gateway, AppState 等
│       ├── server.rs       # build_router, run_server
│       ├── handlers.rs
│       └── state.rs        # AppState, ManagerCommand
│
├── agent-diva-cli/         # CLI 二进制入口
│   └── src/
│       ├── main.rs         # clap 命令解析
│       ├── lib.rs          # pub mod chat_commands, cli_runtime, client, provider_commands
│       └── (tests/ 目录)
│
├── agent-diva-neuron/      # 单轮推理节点抽象
├── agent-diva-service/     # Windows 服务包装
├── agent-diva-migration/   # Python→Rust 迁移工具
└── agent-diva-gui/
    ├── src-tauri/          # Tauri 后端 (Rust)
    └── src/                # Vue 前端
```

### 2.2 每个 Crate 的标准结构

- **`lib.rs`**: 声明所有 `pub mod`，并通过 `pub use` 重导出核心类型
- **`base.rs`**: 定义核心 trait 和错误类型（providers, tools, channels, files 都遵循此模式）
- **子模块**: 用目录 + `mod.rs` 组织（如 `bus/mod.rs`, `config/mod.rs`），或用平级文件（如 `loop_tools.rs`, `loop_turn.rs`）
- **私有模块**: 不加 `pub`（如 `mod http_util;`）

### 2.3 Vue 前端结构

```
agent-diva-gui/src/
├── App.vue                 # 根组件，持有所有全局状态
├── main.ts                 # 应用入口
├── i18n.ts                 # vue-i18n 配置
├── styles.css              # Tailwind 导入 + CSS 变量主题
│
├── api/                    # Tauri invoke 包装 + 类型定义
│   ├── desktop.ts          # 主 API 层 (ConfigStatusReport, McpServerDto 等)
│   ├── providers.ts
│   └── tokenStats.ts
│
├── components/             # 所有组件（无 views/ 目录）
│   ├── ChatView.vue        # 顶层页面组件直接放这里
│   ├── SettingsView.vue
│   ├── ConsoleView.vue
│   ├── NormalMode.vue      # 导航壳
│   ├── ConversationSidebar.vue
│   ├── AppDialogLayer.vue
│   ├── AppToastLayer.vue
│   ├── console/            # 子功能按目录分组
│   │   ├── ConfigEditor.vue
│   │   ├── LogPanel.vue
│   │   └── StatusPanel.vue
│   └── settings/
│       ├── GeneralSettings.vue
│       ├── ProvidersSettings.vue
│       ├── ChannelWizardModal.vue
│       ├── channel-icons.ts      # 辅助 TS 文件放在组件目录内
│       └── channel-platforms.ts
│
├── locales/                # i18n 翻译文件
│   ├── zh.ts               # 中文（默认）
│   └── en.ts               # 英文（回退）
│
└── utils/                  # 通用工具函数
    ├── appDialog.ts
    ├── appToast.ts
    ├── openExternal.ts
    └── localStorageAgentDiva.ts
```

---

## 3. Error Handling Patterns（错误处理模式）

### 3.1 Rust 错误处理

**双轨模式**: 库 crate 用 `thiserror`，应用 crate 用 `anyhow`。

#### 库 Crate — thiserror 自定义错误枚举

```rust
// 每个库 crate 在 base.rs 或 error.rs 中定义自己的错误类型
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("API error: {0}")]
    ApiError(String),
}
```

**规则**:
- 每个库 crate 定义自己的 `XxxError` 枚举，使用 `#[derive(Debug, thiserror::Error)]`
- 用 `#[from]` 实现自动转换（如 `#[from] std::io::Error`, `#[from] reqwest::Error`）
- 手动 `From` impl 用于复杂转换（如 `serde_json::Error` → `Error::Serialization`）
- `agent-diva-core` 的 `Error` 是通用"兜底"类型，其他 crate 各自定义领域错误
- `pub use error::{Error, Result}` 从 crate 根重导出

**各 Crate 错误类型清单**:

| Crate | 错误类型 | 位置 |
|-------|---------|------|
| `agent-diva-core` | `Error` | `src/error.rs` |
| `agent-diva-core` | `SecurityError` | `src/security/error.rs` |
| `agent-diva-providers` | `ProviderError` | `src/base.rs` |
| `agent-diva-providers` | `TranscriptionError` | `src/transcription.rs` |
| `agent-diva-tools` | `ToolError` | `src/base.rs` |
| `agent-diva-tools` | `McpError` | `src/mcp_sdk.rs` |
| `agent-diva-channels` | `ChannelError` | `src/base.rs` |
| `agent-diva-neuron` | `NeuronError` | `src/node.rs` |
| `agent-diva-files` | `FileError` | `src/lib.rs` |

#### 应用 Crate — anyhow

`agent-diva-agent`, `agent-diva-cli`, `agent-diva-manager`, `agent-diva-migration` 直接使用 `anyhow::Result`，不定义自定义错误枚举。

```rust
use anyhow::Result;
async fn do_something() -> Result<()> {
    // ? 自动转换任何实现了 std::error::Error 的类型
    Ok(())
}
```

### 3.2 前端错误处理

- **Tauri invoke 层**: 每个 API 调用用 `try/catch` 包裹，失败时调用 `appToast` 显示错误
- **Toast 通知**: 通过 `src/utils/appToast.ts` 显示用户可见的错误消息
- **Dialog 弹窗**: 通过 `src/utils/appDialog.ts` 显示确认/错误对话框
- **无全局错误边界**: 错误处理分散在各组件中

---

## 4. Testing Patterns（测试模式）

### 4.1 Rust 测试

**主要模式: 内联 `#[cfg(test)]` 模块**（34+ 处），集成测试放在 `tests/` 目录（10 个文件）。

#### 单元测试（内联）

```rust
// 在源文件底部
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_something() {
        let result = some_function().await;
        assert!(result.is_ok());
    }
}
```

**规则**:
- 测试模块放在被测源文件的底部
- `use super::*;` 导入父模块所有内容
- 异步测试用 `#[tokio::test]`
- 测试函数名以 `test_` 开头
- 测试覆盖率最高的 crate: `agent-diva-files`（7/8 文件有测试）、`agent-diva-tools`（9 个模块有测试）

#### 集成测试

```
crate-name/tests/
├── qq_reconnect_integration.rs
├── ollama_streaming.rs
├── config_commands.rs
└── ...
```

**测试工具依赖**（workspace 级别）:
- `mockito` — HTTP mock
- `wiremock` — 更灵活的 HTTP mock
- `tokio-test` — 异步测试辅助
- `tempfile` — 临时文件/目录

### 4.2 前端测试

**无测试基础设施**。没有 vitest/jest 配置，没有测试文件，没有测试依赖。唯一的 Storybook story 文件 (`SettingsView.stories.ts`) 被 tsconfig 排除。

---

## 5. Development Workflow（开发流程）

### 5.1 justfile 命令

| 命令 | 用途 | 命令内容 |
|------|------|----------|
| `just build` | 构建所有 crate | `cargo build --all` |
| `just build-release` | Release 构建 | `cargo build --all --release` |
| `just test` | 运行所有测试 | `cargo test --all` |
| `just check` | Clippy lint | `cargo clippy --all -- -D warnings` |
| `just fmt` | 格式化代码 | `cargo fmt --all` |
| `just fmt-check` | 检查格式（不修改） | `cargo fmt --all -- --check` |
| `just ci` | 完整 CI 流水线 | `fmt-check` → `check` → `test` |
| `just doc` | 生成文档 | `cargo doc --all --no-deps` |
| `just run <ARGS>` | 运行 CLI | `cargo run --package agent-diva-cli -- <ARGS>` |
| `just install` | 安装 CLI 二进制 | `cargo install --path agent-diva-cli` |
| `just clean` | 清理构建产物 | `cargo clean` |
| `just audit` | 安全审计 | `cargo audit` |
| `just bench` | 运行基准测试 | `cargo bench --all` |

### 5.2 CI 配置

- **触发**: push 到 `main`/`develop`，PR 到 `main`/`develop`，`v*.*.*` 标签
- **路径过滤**: 只在 `**/*.rs`, `**/Cargo.toml`, `Cargo.lock`, `justfile`, `agent-diva-gui/**` 变更时触发
- **三平台矩阵**: ubuntu-latest, windows-latest, macos-latest
- **Job 依赖**: `rust-check` → `gui-build` → `release`
- **测试可选**: 默认关闭，通过 `workflow_dispatch` 的 `run_tests` 输入手动开启
- **覆盖率**: `cargo-tarpaulin` + Codecov，仅手动触发

### 5.3 Commit 规范

**无强制规范**。没有 commitlint、commitizen 或 conventional commits 配置。从 git 历史看，使用自由格式的 commit message。

### 5.4 版本发布

- 通过 `v*.*.*` 标签触发
- CI 自动构建三平台 GUI 产物（AppImage, deb, dmg, msi, exe）
- 使用 `softprops/action-gh-release` 创建 GitHub Release
- Workspace 版本统一管理: `Cargo.toml` 中的 `version = "0.4.9"`

---

## 6. Import/Export Conventions（导入导出约定）

### 6.1 Rust 导入导出

#### pub use 重导出模式

每个 crate 的 `lib.rs` 通过 `pub use` 重导出核心类型，使消费者不需要写深层路径:

```rust
// agent-diva-core/src/lib.rs
pub use attachment::FileAttachment;
pub use error::{Error, Result};

// agent-diva-manager/src/lib.rs
pub use manager::Manager;
pub use runtime::{run_local_gateway, start_embedded_gateway_runtime, ...};
pub use server::{build_router, run_server, run_server_with_listener};
pub use state::{ApiRequest, AppState, ManagerCommand};
```

**规则**:
- `lib.rs` 中声明所有 `pub mod`，然后 `pub use` 重导出最重要的类型
- 子模块（如 `bus/mod.rs`）也可以有自己的 `pub use`
- Trait 定义放在 `base.rs`（providers, tools, channels, files 都遵循此模式）

#### 依赖导入风格

```rust
// 标准库在前，外部 crate 在后，本项目 crate 最后
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use agent_diva_core::{Error, Result};
```

### 6.2 TypeScript/Vue 导入导出

**使用相对路径，无 `@/` 别名**:

```vue
<script setup lang="ts">
// 同目录组件
import ChatView from './ChatView.vue'
// 子目录组件
import McpSettings from './settings/McpSettings.vue'
// API 层（相对路径）
import { getConfigStatus } from '../../api/desktop'
// 工具函数
import { clearAgentDivaLocalStorage } from '../../utils/localStorageAgentDiva'
// 外部包
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { Send, Square } from 'lucide-vue-next'
</script>
```

**API 层导出模式**:

```typescript
// src/api/desktop.ts — interface 直接 export，函数直接 export
export interface McpServerDto { ... }
export async function getConfigStatus(): Promise<ConfigStatusReport> {
  return invoke('get_config_status');
}
```

---

## 7. 前端特定模式

### 7.1 Vue 3 Composition API

**100% 使用 `<script setup lang="ts">`**，无 Options API。

```vue
<template>
  <div>{{ t('key') }}</div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'

// Props（typed）
const props = defineProps<{
  messages: Message[]
  isTyping: boolean
}>()

// Emits（typed call-signature）
const emit = defineEmits<{
  (e: 'send', content: string, attachments?: string[]): void
  (e: 'clear'): void
}>()

// Local state
const input = ref('')
const filtered = computed(() => props.messages.filter(...))

// Lifecycle
onMounted(() => { ... })
onUnmounted(() => { ... })
</script>

<style scoped>
/* 组件特有样式，尽量用 Tailwind */
</style>
```

### 7.2 状态管理

**无 Pinia/Vuex**。状态管理模式:

1. **App.vue 持有全局状态** — messages, config, sessions 等
2. **Props down, events up** — 子组件通过 props 接收，通过 emit 向上传递
3. **`defineExpose()`** — 暴露方法给父组件通过 template ref 调用
4. **`localStorage`** — 持久化偏好设置（key 集中在 `localStorageAgentDiva.ts`）

```typescript
// src/utils/localStorageAgentDiva.ts
export const LOCALE_STORAGE_KEY = 'agent-diva-locale'
export const SAVED_MODELS_KEY = 'agent-diva-saved-models'
// ... 所有 localStorage key 集中定义
```

### 7.3 i18n 实现

- **库**: vue-i18n v9，Composition API 模式 (`legacy: false`)
- **语言**: 中文 (`zh`) 为默认，英文 (`en`) 为回退
- **文件**: `src/locales/zh.ts`, `src/locales/en.ts`（纯 TS 对象）
- **持久化**: 用户语言偏好存 localStorage

```vue
<script setup lang="ts">
const { t } = useI18n()
</script>
<template>
  <p>{{ t('settings.general.title') }}</p>
  <p>{{ t('app.error', { detail: errorMsg }) }}</p>
</template>
```

### 7.4 CSS 方案

**Tailwind CSS 3.4 + CSS Custom Properties + scoped styles**

```vue
<template>
  <!-- 主要用 Tailwind 工具类 -->
  <div class="flex items-center gap-2 p-4 bg-yandere-50 rounded-lg">
    ...
  </div>
</template>

<style scoped>
/* 组件特有动画/深度选择器 */
:deep(.markdown-body) { ... }
@keyframes slideIn { ... }
</style>
```

**主题系统**: 通过 `data-theme` 属性切换 CSS 变量

```css
/* src/styles.css */
:root[data-theme="love"] { --bg-primary: #fff0f5; ... }
:root[data-theme="dark"] { --bg-primary: #1a1a2e; ... }
:root[data-theme="default"] { --bg-primary: #ffffff; ... }
```

**自定义色板**: `tailwind.config.js` 中定义 `yandere` 色系（粉红/玫红）。

---

## 8. 关键架构模式速查

### 8.1 Trait-per-Crate 模式

每个库 crate 在 `base.rs` 中定义一个核心 trait:

| Crate | Trait | 位置 |
|-------|-------|------|
| `agent-diva-providers` | `LLMProvider` | `src/base.rs` |
| `agent-diva-tools` | `Tool` | `src/base.rs` |
| `agent-diva-channels` | `BaseChannel` / `ChannelHandler` | `src/base.rs` |
| `agent-diva-neuron` | `NeuronNode` | `src/node.rs` |
| `agent-diva-files` | `StorageBackend` | `src/backend.rs` |

### 8.2 数据流

```
Channel Handler → Message Bus (inbound) → Agent Loop
  → Context Builder → LLM Provider → Tool Execution
  → Message Bus (outbound) → Channel Handler (response)
```

### 8.3 代码格式化规则

```toml
# rustfmt.toml
edition = "2021"
max_width = 100        # 行宽 100 字符
tab_spaces = 4         # 4 空格缩进
hard_tabs = false      # 不用 tab
reorder_imports = true # 自动重排 import
```

```toml
# clippy.toml
msrv = "1.80.0"
cognitive-complexity-threshold = 25
type-complexity-threshold = 250
```

**Clippy 严格模式**: `cargo clippy --all -- -D warnings`（所有 warning 视为 error）

### 8.4 异步运行时

- **Tokio** 多线程运行时
- 消息传递: `tokio::sync::mpsc`
- 并发: `tokio::spawn`
- Trait 异步方法: `async-trait` crate

### 8.5 日志

- **tracing** crate（非 `log`）
- `info!()`, `debug!()`, `warn!()`, `error!()` 宏
- `tracing-subscriber` 配置 env-filter 和 local-time
- 设置日志级别: `RUST_LOG=debug`

### 8.6 序列化

- **serde** + `derive` feature
- JSON: `serde_json`
- YAML: `serde_yaml`
- 配置文件格式: JSON (`~/.agent-diva/config.json`)

### 8.7 HTTP 客户端

- **reqwest** + `rustls-tls`（不用 native-tls）
- WebSocket: `tokio-tungstenite`
- GUI 端: reqwest + `.no_proxy()`（避免系统代理干扰 localhost）

### 8.8 数据库

- **rusqlite** (bundled) — 轻量嵌入式
- **sqlx** (sqlite + migrate) — 异步 + 编译时检查

---

## 9. AI Agent 工作规则

### 9.1 修改 Rust 代码时

1. **文件命名**: 新文件用 `snake_case.rs`
2. **模块声明**: 在父模块的 `mod.rs` 或 `lib.rs` 中添加 `pub mod xxx;`
3. **错误处理**: 库 crate 用 `thiserror` 定义 `XxxError`，应用 crate 用 `anyhow::Result`
4. **Trait 实现**: 新 provider/channel/tool 实现对应 trait
5. **重导出**: `lib.rs` 中 `pub use` 核心类型
6. **格式化**: 运行 `just fmt` 或 `cargo fmt --all`
7. **Lint**: 运行 `just check` 或 `cargo clippy --all -- -D warnings`
8. **测试**: 在源文件底部添加 `#[cfg(test)] mod tests { ... }`
9. **异步**: 用 `#[tokio::test]` 测试异步函数
10. **Provider Model-ID**: 原生端点保持原始 model ID，不自动添加 LiteLLM 前缀

### 9.2 修改前端代码时

1. **组件命名**: `PascalCase.vue`，模板中用 `<PascalCase />`
2. **API 风格**: `<script setup lang="ts">`，不用 Options API
3. **状态**: 本地 `ref()` + props/events，不引入 Pinia
4. **样式**: Tailwind 工具类为主，`<style scoped>` 为辅
5. **导入**: 相对路径，不用 `@/` 别名
6. **类型**: interface 在组件内定义或从 `api/desktop.ts` 导入
7. **i18n**: 新文本同时添加到 `locales/zh.ts` 和 `locales/en.ts`
8. **Typed emits**: 用 call-signature 语法 `defineEmits<{ ... }>()`
9. **Typed props**: 用 `defineProps<{ ... }>()` 或 `defineProps<Props>()`

### 9.3 提交前检查清单

```bash
just ci          # 等价于: fmt-check → clippy → test
# 或分步:
just fmt         # 格式化
just check       # clippy
just test        # 测试
```

### 9.4 常见陷阱

- **GUI 代理问题**: reqwest 必须用 `.no_proxy()` 避免系统代理干扰 localhost
- **文件路径**: 上传和读取必须用相同的路径计算 (`dirs::data_local_dir()`)
- **Windows 编码**: Shell 管道输出需解码 UTF-8 或 GB18030
- **MSRV**: 最低支持 Rust 1.80.0，不要用更新版本的特性
