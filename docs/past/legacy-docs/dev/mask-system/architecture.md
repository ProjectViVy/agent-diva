---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
workflowType: 'architecture'
lastStep: 8
status: 'complete'
completedAt: '2026-06-10'
inputDocuments:
  - docs/dev/mask-system/prd.md
  - docs/dev/mask-system/.decision-log.md
  - docs/project-context.md
project_name: 'Agent-Diva Mask System'
user_name: '大湿'
date: '2026-06-10'
---

# Architecture Decision Document — Mask System

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

---

## 1. Project Context Summary

**项目**：Agent-Diva Pro — 面具系统（Mask System & Enhanced Sub-Agent Architecture）
**范围**：提示词拼接 + 工具限制（harness 工程）+ 子 Agent 隔离
**Stakes**：Internal tool
**技术栈**：Rust workspace（13 crates），Tauri GUI，SQLite

### 核心架构约束

- Agent-Diva 只有一个统一人格（松本/diva）
- 面具是行为层/cosplay，不是身份层
- 面具之间不通信
- 同时只戴一个面具
- 子 agent 无人格上下文
- 工具限制在 runtime 级强制执行

### 关键 Crate 依赖

| Crate | 与面具系统的关系 |
|-------|----------------|
| `agent-diva-core` | Config schema（MaskConfig、ToolLimits）、ContextBuilder |
| `agent-diva-agent` | AgentLoop、SubagentManager、ToolAssembly |
| `agent-diva-tools` | ToolRegistry、for_subagent() |
| `agent-diva-gui` | Header 切换器、Settings 面板、编辑器 |
| `agent-diva-cli` | /mask 命令处理 |

### PRD 概要

- **30 条 FR**，分 5 组
- **3 Epic / 13 Story**
- **12 条已确认决策**（无遗留 open questions）
- 面具文件格式：Markdown + YAML frontmatter，存储在 `workspace/masks/`
- Prompt 拼接：方案 A（面具在顶层）
- 工具限制：`global ∩ allow − deny`，子 agent `child ⊆ parent`
- 切换时：compress → 注入切换消息 → 注入面具 prompt

---

## 3. Starter Template Evaluation

**不适用**。agent-diva-pro 是已存在的 Rust workspace（13 crates），面具系统是在现有架构上添加的功能模块。

现有架构基础已就位：Config schema、ContextBuilder、ToolRegistry、SubagentManager、GUI (Tauri + Vue 3)、SQLite (sqlx)。

决策：直接在现有架构上扩展，不需要 starter template。

---

## 4. Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions（阻塞实现）：**
- A-1: 面具状态持久化 → 文件系统扫描
- A-2: MaskLoader 代码位置 → `agent-diva-agent`
- A-3: ContextBuilder 注入方式 → MaskPromptComposer 中间层
- A-4: 工具限制实现层 → ToolRegistry 级过滤

**Important Decisions（影响架构）：**
- A-5: 子 agent 能力收窄 → 独立 ToolPolicy
- A-6: GUI 通信 → Tauri IPC

### Data Architecture

**A-1: 面具状态持久化**
- **决策**：文件系统扫描（每次启动扫描 `workspace/masks/` 目录）
- **理由**：简单直接，无需额外 DB，面具文件本身就是数据源
- **影响**：FR-1, FR-2

### MaskLoader Architecture

**A-2: MaskLoader 代码位置**
- **决策**：放在 `agent-diva-agent` crate（与 ContextBuilder 同 crate）
- **理由**：减少跨 crate 依赖，MaskLoader 和 ContextBuilder 紧密协作
- **影响**：`agent-diva-agent/src/mask/` 新模块

### Prompt Composition

**A-3: ContextBuilder 注入方式**
- **决策**：新增 `MaskPromptComposer` 中间层
- **理由**：解耦面具逻辑与 ContextBuilder，可独立测试
- **实现**：ContextBuilder 调用 `MaskPromptComposer::compose()` 获取面具 prompt 片段
- **影响**：FR-9, FR-10

### Tool Policy Enforcement

**A-4: 工具限制实现层**
- **决策**：ToolRegistry 级过滤
- **理由**：agent 看不到被禁工具，最安全（runtime 级强制，非 prompt-only）
- **实现**：`ToolRegistry::with_mask_policy()` 创建过滤后的 registry
- **影响**：FR-13~16

### Sub-Agent Capability Resolution

**A-5: 子 agent 能力收窄实现**
- **决策**：独立 ToolPolicy，运行时解析
- **理由**：灵活性更高，支持子 agent 独立的 allow/deny 配置
- **实现**：`ToolPolicy::resolve(parent_tools, child_config)` → reduced tool set
- **影响**：FR-19~21

### GUI Communication

**A-6: GUI 通信机制**
- **决策**：Tauri IPC（已有模式）
- **理由**：复用现有架构，无需引入新的通信层
- **影响**：FR-25~30

### Decision Impact Analysis

**Implementation Sequence：**
1. MaskLoader + MaskFile 解析（A-1, A-2）
2. MaskPromptComposer（A-3）
3. ToolRegistry 过滤（A-4）
4. ToolPolicy + 子 agent 收窄（A-5）
5. ContextBuilder 集成（A-3 + A-4）
6. GUI 集成（A-6）

**Cross-Component Dependencies：**
- A-2(B) + A-3(B)：`agent-diva-agent` 新增 `mask/` 模块
- A-4(A)：`agent-diva-tools` 的 ToolRegistry 需要支持按 MaskConfig 过滤
- A-5(B)：SubagentManager 从 ToolPolicy 解析 reduced tool set

---

## 5. Implementation Patterns & Consistency Rules

### Naming Patterns

**Rust 命名（遵循 project-context.md）：**

| 元素 | 规则 | 示例 |
|------|------|------|
| 模块名 | `snake_case` | `mask_loader`, `mask_prompt_composer`, `tool_policy` |
| 类型/结构体 | `PascalCase` | `MaskFile`, `MaskConfig`, `MaskRegistry`, `ToolPolicy` |
| 枚举 | `PascalCase` | `MaskError`, `AgentMode` |
| 函数 | `snake_case` | `load_mask_file`, `compose_mask_prompt`, `resolve_tool_policy` |
| 常量 | `SCREAMING_SNAKE_CASE` | `DEFAULT_MASK_NAME`, `MASKS_DIR_NAME` |
| Trait | `PascalCase`（无 I/T 前缀） | `MaskLoader`, `ToolPolicyResolver` |

**文件命名：**

| 类型 | 规则 | 示例 |
|------|------|------|
| 源文件 | `snake_case.rs` | `mask_file.rs`, `mask_registry.rs`, `tool_policy.rs` |
| 模块目录 | `snake_case/` | `mask/`（含 `mod.rs`） |
| 测试文件 | `tests/<描述>.rs` | `tests/mask_loader_integration.rs` |

### Structure Patterns

**新模块组织（`agent-diva-agent/src/mask/`）：**

```
agent-diva-agent/src/mask/
├── mod.rs                    # 公开 API
├── mask_file.rs              # MaskFile 结构体 + YAML 解析
├── mask_registry.rs          # MaskRegistry（扫描目录、缓存）
├── mask_prompt_composer.rs   # MaskPromptComposer（prompt 拼接）
├── tool_policy.rs            # ToolPolicy（allow/deny 计算）
└── error.rs                  # MaskError 枚举
```

### Format Patterns

**Frontmatter YAML Schema：**

```yaml
---
name: string           # 必填，面具显示名
icon: string           # 可选，emoji 或图标名
description: string    # 可选，面具描述
model: string          # 可选，覆盖全局模型
subagent_defaults:     # 可选，子 agent 默认配置
  model: string
  max_iterations: number
tool_limits:           # 可选，工具限制
  allow: [string]      # 白名单
  deny: [string]       # 黑名单
---
```

**错误格式：**

```rust
#[derive(Debug, thiserror::Error)]
pub enum MaskError {
    #[error("Mask not found: {0}")]
    NotFound(String),
    #[error("Invalid mask file {path}: {reason}")]
    InvalidFile { path: String, reason: String },
    #[error("Invalid frontmatter in {path}: {reason}")]
    InvalidFrontmatter { path: String, reason: String },
}
```

### Communication Patterns

**面具切换事件（Tauri IPC）：**

```rust
#[derive(Serialize, Deserialize)]
pub struct MaskSwitchEvent {
    pub from: Option<String>,  // None = 从默认面具切换
    pub to: String,            // 目标面具名
    pub timestamp: i64,
}
```

### Process Patterns

**面具加载失败处理：**
- 单个面具文件解析失败 → 跳过 + 日志警告 + 继续加载其他面具
- 目录不存在 → 返回空列表 + 提示用户创建
- frontmatter 格式错误 → 跳过 + 具体错误信息

**面具切换失败处理：**
- 目标面具不存在 → 返回错误，不切换
- 上下文压缩失败 → 警告但继续切换
- 切换消息注入失败 → 警告但继续切换

### Enforcement Guidelines

**所有 AI agent 必须：**
1. 遵循 project-context.md 中的命名约定
2. 面具相关类型放在 `agent-diva-agent/src/mask/` 模块
3. 使用 `MaskError` 统一错误类型
4. frontmatter 解析使用 `serde_yaml`
5. 工具限制在 ToolRegistry 级强制（非 prompt-only）

---

## 6. Project Structure & Boundaries

### 需求到结构映射

| Epic/FR 组 | 位置 | 说明 |
|------------|------|------|
| 面具管理 (FR-1~8) | `agent-diva-agent/src/mask/` | MaskLoader + MaskRegistry |
| Prompt 拼接 (FR-9~12) | `agent-diva-agent/src/mask/mask_prompt_composer.rs` | MaskPromptComposer |
| 工具限制 (FR-13~18) | `agent-diva-tools/src/registry.rs` 扩展 | `with_mask_policy()` |
| 子 Agent 隔离 (FR-19~24) | `agent-diva-agent/src/mask/tool_policy.rs` | ToolPolicy |
| GUI 集成 (FR-25~30) | `agent-diva-gui/src/` | Header + Settings |

### 新增文件结构

```
agent-diva-pro/
├── agent-diva-core/
│   └── src/config/
│       └── schema.rs              # [修改] 新增 MaskConfig, ToolLimits 类型
│
├── agent-diva-agent/
│   └── src/
│       ├── mask/                   # [新增] 面具模块
│       │   ├── mod.rs              # 公开 API
│       │   ├── mask_file.rs        # MaskFile 结构体 + YAML 解析
│       │   ├── mask_registry.rs    # MaskRegistry（扫描目录、缓存）
│       │   ├── mask_prompt_composer.rs  # MaskPromptComposer（prompt 拼接）
│       │   ├── tool_policy.rs      # ToolPolicy（allow/deny 计算）
│       │   └── error.rs            # MaskError 枚举
│       ├── context.rs              # [修改] 调用 MaskPromptComposer
│       └── subagent.rs             # [修改] 使用 ToolPolicy 收窄子 agent
│
├── agent-diva-tools/
│   └── src/
│       └── registry.rs             # [修改] 新增 with_mask_policy() 方法
│
├── agent-diva-gui/
│   └── src/
│       ├── components/
│       │   ├── MaskSwitcher.vue    # [新增] Header 面具切换下拉
│       │   └── settings/
│       │       └── MaskSettings.vue # [新增] 面具设置面板
│       └── composables/
│           └── useMask.ts          # [新增] 面具状态管理 composable
│
└── agent-diva-cli/
    └── src/
        └── chat_commands.rs        # [修改] 新增 /mask 命令处理
```

### 架构边界

**Crate 边界：**
- `agent-diva-core`：只提供类型定义（MaskConfig, ToolLimits），不包含逻辑
- `agent-diva-agent`：面具系统的核心逻辑（加载、注册、拼接、策略）
- `agent-diva-tools`：工具限制的执行层（ToolRegistry 过滤）
- `agent-diva-gui`：面具系统的 UI 层

**通信模式：**
- `agent-diva-agent` → `agent-diva-core`：读取 MaskConfig 类型
- `agent-diva-agent` → `agent-diva-tools`：调用 `ToolRegistry::with_mask_policy()`
- `agent-diva-gui` → `agent-diva-agent`：Tauri IPC 调用面具 API

### 数据流

```
用户输入 /mask wear researcher
    ↓
CLI 解析命令
    ↓
MaskRegistry.get("researcher") → MaskFile
    ↓
MaskPromptComposer.compose(mask_file) → prompt 片段
    ↓
ContextBuilder.build_system_prompt(mask_prompt) → 完整 system prompt
    ↓
ToolRegistry::with_mask_policy(tool_limits) → 过滤后的工具集
    ↓
AgentLoop 使用新的 prompt + 工具集运行
    ↓
Tauri IPC → GUI 更新 Header 显示
```

---

## 7. Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility：**
- ✅ A-1(文件系统扫描) + A-2(agent-diva-agent) + A-3(MaskPromptComposer) — 一致
- ✅ A-4(ToolRegistry 级过滤) + A-5(独立 ToolPolicy) — 互补
- ✅ A-6(Tauri IPC) — 与现有 GUI 架构一致

**Pattern Consistency：** ✅ 命名、模块、错误处理均遵循 project-context.md

**Structure Alignment：** ✅ 新模块位置与现有 crate 职责划分一致

### Requirements Coverage Validation ✅

| Epic/FR 组 | 架构支持 | 状态 |
|------------|----------|------|
| 面具管理 (FR-1~8) | MaskLoader + MaskRegistry | ✅ |
| Prompt 拼接 (FR-9~12) | MaskPromptComposer + ContextBuilder | ✅ |
| 工具限制 (FR-13~18) | ToolPolicy + ToolRegistry::with_mask_policy() | ✅ |
| 子 Agent 隔离 (FR-19~24) | ToolPolicy + SubagentManager | ✅ |
| GUI 集成 (FR-25~30) | MaskSwitcher + MaskSettings + useMask | ✅ |

**NFR 覆盖：** ✅ 切换延迟、runtime 强制、可测试性、向后兼容

### Implementation Readiness ✅

- ✅ 6 个架构决策全部有明确选择和理由
- ✅ 新增/修改文件结构完整定义
- ✅ 命名、结构、格式、通信、流程模式全部定义

### Gap Analysis

**Critical Gaps：** 无

**Important Gaps：**
- 面具热加载（/mask reload）实现细节未展开
- GUI 编辑器交互未详细设计（P2）

### Architecture Completeness Checklist

**Requirements Analysis**
- [x] Project context thoroughly analyzed
- [x] Scale and complexity assessed
- [x] Technical constraints identified
- [x] Cross-cutting concerns mapped

**Architectural Decisions**
- [x] Critical decisions documented with versions
- [x] Technology stack fully specified
- [x] Integration patterns defined
- [x] Performance considerations addressed

**Implementation Patterns**
- [x] Naming conventions established
- [x] Structure patterns defined
- [x] Communication patterns specified
- [x] Process patterns documented

**Project Structure**
- [x] Complete directory structure defined
- [x] Component boundaries established
- [x] Integration points mapped
- [x] Requirements to structure mapping complete

### Architecture Readiness Assessment

**Overall Status：READY FOR IMPLEMENTATION**

**Confidence Level：High** — 16/16 checklist 通过，0 Critical Gaps

**Key Strengths：**
- 架构决策与现有 workspace 完全兼容
- 需求到结构映射清晰
- 实现模式具体可操作

### Implementation Handoff

**AI Agent Guidelines：**
- 遵循本文档中所有架构决策
- 使用统一的实现模式
- 尊重项目结构和边界

**First Implementation Priority：**
1. `agent-diva-core/src/config/schema.rs` — 新增 MaskConfig, ToolLimits 类型
2. `agent-diva-agent/src/mask/` — 创建面具模块（mask_file.rs, mask_registry.rs, ...）

### Requirements Overview

**Functional Requirements（30 条，5 组）：**

| 组 | FR 数 | 架构含义 |
|----|-------|----------|
| 面具管理 | FR-1~8 | 文件 I/O + 解析 + 注册表 + 生命周期管理 |
| Prompt 拼接 | FR-9~12 | ContextBuilder 修改 + 上下文压缩集成 |
| 工具限制 | FR-13~18 | ToolRegistry 扩展 + runtime enforcement + AgentMode |
| 子 Agent 隔离 | FR-19~24 | SubagentManager 修改 + 能力收窄 + 并行执行 |
| GUI 集成 | FR-25~30 | Header 组件 + Settings 面板 + 编辑器 + 事件流 |

**Non-Functional Requirements：**
- 面具切换延迟 < 1s（含 compress）
- 工具限制 100% runtime 强制（非 prompt-only）
- 子 agent 能力收窄 100% 可测试
- 面具文件格式向后兼容

**Scale & Complexity：**
- Primary domain: Rust backend + Tauri desktop GUI
- Complexity level: Medium
- Estimated architectural components: 6（MaskLoader、MaskRegistry、ContextBuilder 扩展、ToolPolicy、SubagentPolicy、GUI 组件）

### Technical Constraints & Dependencies

| 约束 | 来源 | 影响 |
|------|------|------|
| Rust workspace 13 crates | 现有架构 | 修改需跨 crate 协调 |
| ContextBuilder 是 prompt 组装核心 | `agent-diva-agent/src/context.rs` | 面具 prompt 注入点 |
| ToolRegistry 支持 `for_subagent()` | `agent-diva-tools/src/registry.rs` | 工具限制扩展点 |
| SubagentManager 管理子 agent 生命周期 | `agent-diva-agent/src/subagent.rs` | 能力收窄扩展点 |
| Config schema 已有 `SubagentToolsConfig` | `agent-diva-core/src/config/schema.rs` | MaskConfig 定义位置 |
| GUI 使用 Vue 3 + Tauri | `agent-diva-gui/` | 前端组件开发 |
| SQLite 已就位（sqlx） | workspace dep | 面具状态持久化（可选） |

### Cross-Cutting Concerns

1. **安全性**：工具限制必须在 runtime 级强制，不能仅靠 prompt
2. **可测试性**：`child ⊆ parent` 需要确定性测试覆盖
3. **向后兼容**：无面具时行为不变（默认面具"我就是我"）
4. **上下文管理**：切换时 compress 需要与现有 session 系统协调
5. **GUI 实时性**：面具切换后 GUI 需要同步更新
