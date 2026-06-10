---
title: "Mask System & Enhanced Sub-Agent Architecture — PRD"
status: final
created: 2026-06-10
updated: 2026-06-10
author: Agent Matsumoto (BMad PRD workflow, recovered from 2026-06-04 session)
---

# Mask System & Enhanced Sub-Agent Architecture — PRD

> **项目**：Agent-Diva Pro
> **范围**：面具系统（提示词拼接 + 工具限制 + 子 Agent 隔离）
> **Stakes**：Internal tool
> **来源**：从 2026-06-04 session 记录恢复，BMad Rubric 评级 Good

---

## 1. Vision

Agent-Diva 只有一个统一人格（松本/diva）。面具系统是一层**提示词拼接 + 工具限制（harness）**，让同一个 agent 在不同任务场景下表现出不同的行为模式，同时通过 runtime 级别的工具限制确保安全边界。

**核心哲学**：
- 自始至终只有一个 diva，面具不定义"另一个智能体是谁"
- 面具是行为层/cosplay，不是身份层
- 面具之间不通信
- 同时只戴一个面具，可随时切换、可临时脱下
- 子 agent 无人格上下文，只拿任务

**比喻**：主舞台上，舞者戴着面具，但大家还能认得出来这是某个知名的演员；他的伴舞都是看不清脸的白色面具人。

---

## 2. Goals

| # | 目标 |
|---|------|
| G1 | 用户可以通过 `/mask` 命令切换不同的行为模式 |
| G2 | 每个面具可以定义独立的提示词、工具限制、模型选择 |
| G3 | 面具切换时自动压缩上下文，确保干净的角色切换 |
| G4 | 子 agent 不继承 diva 人格，只拿任务上下文 |
| G5 | 子 agent 的工具集从父 agent 收窄（child ⊆ parent） |
| G6 | 面具系统通过 GUI 可视化管理 |

### 2.1 Non-Goals

| 不在范围内 | 理由 |
|-----------|------|
| 运行时动态创建面具 | 面具通过文件定义，不需要运行时 API |
| 无需 `/mask reload` 的热加载 | 手动 reload 足够 |
| 同时佩戴多个面具 | 复杂度高，收益低 |
| 面具版本管理/回滚 | 文件系统 + git 足够 |
| 子 agent 间直接通信 | 违背隔离原则 |
| 面具 marketplace/分享 | 远期功能 |
| system-v 级多智能体网络 | 留给 system-v |

### 2.2 Assumptions

| 假设 | 风险 | 缓解 |
|------|------|------|
| `workspace/masks/` 目录存在且可读 | 目录不存在时面具列表为空 | `/mask list` 输出提示 |
| 面具文件名唯一（路径作为 key） | 同名文件覆盖 | 加载时警告 |
| skills 可安全卸载/重载 | 面具切换时需要重新组装工具 | 已有 `for_subagent()` 机制 |
| frontmatter 可手工编辑 | 格式错误导致面具不可用 | 校验 + 错误提示 |
| prompt 长度合理（< 4K tokens） | 超长面具 prompt 挤占上下文 | 截断警告 |
| tokio cancel 行为安全 | 异步任务取消可能导致状态不一致 | 已有 `LoopGuard` |

---

## 3. Architecture Overview

### 3.1 面具文件格式

存储在 `workspace/masks/` 目录下，支持嵌套子目录。格式：**Markdown + YAML frontmatter**。

```yaml
---
name: "研究员"
icon: "🔍"
description: "专注调研与分析"
model: "deepseek-chat"              # 可选，覆盖全局默认
subagent_defaults:                   # 子 agent 默认配置
  model: "gpt-4o-mini"
  max_iterations: 10
tool_limits:
  allow: [read_file, search_files, web_search, web_extract]
  deny: [terminal, write_file]
---

你是一个专注调研与分析的研究员。擅长信息收集、对比分析、文档整理。
```

### 3.2 Prompt 组合层级

采用**方案 A：面具在顶层**，cosplay 效果最强。

```
系统基础 prompt
  ↓
+ 松本核心 prompt（SOUL.md + IDENTITY.md）
  ↓
+ 面具 prompt（从 .md 文件 body 部分读取）
  ↓
= 最终 system prompt → 发送给 LLM
```

### 3.3 模型解析链

优先级从高到低：

```
spawn 显式指定 → 面具 subagent_defaults.model → 面具 model → 全局默认
```

### 3.4 工具限制计算

```
effective_tools = global_builtin ∩ allow − deny
子 agent 工具 = parent_effective ∩ child_allow − child_deny  (child ⊆ parent)
```

### 3.5 面具生命周期

```
/mask list          → 列出可用面具
/mask wear <name>   → 佩戴面具（先 compress → 注入切换消息 → 注入面具 prompt）
/mask off           → 脱下面具（回到默认"我就是我"）
/mask status        → 查看当前面具
/mask reload        → 重新加载面具文件
```

**切换时的上下文管理**：
1. 执行一次上下文压缩（`/compress`）
2. 注入切换系统消息（"面具已切换为 XXX"）
3. 注入面具 prompt
4. 目的：让面具拿到更干净、聚焦的上下文

### 3.6 默认面具

内置默认面具 `"我就是我"`：
- 不可删除
- 等同无面具
- 无额外 prompt 注入
- 无工具限制
- `/mask off` 回到此状态

### 3.7 子 Agent 行为

- 子 agent **不继承** diva 人格上下文（SOUL.md、IDENTITY.md 不注入）
- 子 agent 唯一上下文来源：父 agent 传入的任务
- 子 agent 可使用与当前面具不同的模型
- 子 agent 工具限制方向：`child ⊆ parent`（参考 OpenFang capability 继承）
- Batch spawn：返回全部结果（含失败），由主 agent 决定

---

## 4. Features & Requirements

### 4.1 面具管理（FR-1 ~ FR-8）

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-1 | 系统能从 `workspace/masks/` 目录加载面具文件 | P0 |
| FR-2 | 面具文件使用 Markdown + YAML frontmatter 格式 | P0 |
| FR-3 | frontmatter 支持 name、icon、description、model、subagent_defaults、tool_limits 字段 | P0 |
| FR-4 | 支持嵌套子目录（如 `coding/rust-coder.md`） | P1 |
| FR-5 | 内置默认面具"我就是我"，不可删除 | P0 |
| FR-6 | `/mask list` 列出所有可用面具 | P0 |
| FR-7 | `/mask wear <name>` 切换面具 | P0 |
| FR-8 | `/mask off` 回到默认面具 | P0 |

### 4.2 Prompt 拼接（FR-9 ~ FR-12）

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-9 | 面具 prompt 拼接到 system prompt 顶层（方案 A） | P0 |
| FR-10 | 切换面具时先压缩上下文再注入 | P0 |
| FR-11 | 切换时注入系统消息通知 LLM | P0 |
| FR-12 | 默认面具不注入额外 prompt | P0 |

### 4.3 工具限制（FR-13 ~ FR-18）

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-13 | 面具可定义 `tool_limits.allow` 和 `tool_limits.deny` | P0 |
| FR-14 | 有效工具集 = `global ∩ allow − deny` | P0 |
| FR-15 | Reviewer 面具可启用只读模式（AgentMode::Assist） | P1 |
| FR-16 | 工具限制在 runtime 级强制执行（非纯 prompt） | P0 |
| FR-17 | 面具可覆盖全局模型设置 | P1 |
| FR-18 | 模型解析链：spawn > subagent_defaults > mask model > global | P1 |

### 4.4 子 Agent 隔离（FR-19 ~ FR-24）

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-19 | 子 agent 不继承 diva 人格（SOUL/IDENTITY 不注入） | P0 |
| FR-20 | 子 agent 唯一上下文来源 = 父 agent 传入的任务 | P0 |
| FR-21 | 子 agent 工具集 = `parent_effective ∩ child_allow − child_deny` | P0 |
| FR-22 | 子 agent 间不直接通信 | P0 |
| FR-23 | Batch spawn 返回全部结果（含失败） | P1 |
| FR-24 | 子 agent 可使用与当前面具不同的模型 | P1 |

### 4.5 GUI 集成（FR-25 ~ FR-30）

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-25 | Header 区域显示当前面具状态 | P1 |
| FR-26 | Header 提供面具切换下拉菜单 | P1 |
| FR-27 | Settings 页面展示面具列表和详情 | P1 |
| FR-28 | 支持面具文件编辑器（基础模式 + 高级 YAML 模式） | P2 |
| FR-29 | 预置面具：researcher、coder、reviewer、assistant | P1 |
| FR-30 | 面具切换时 GUI 实时更新状态 | P1 |

---

## 5. Decisions Log

| # | 问题 | 决策 | 理由 |
|---|------|------|------|
| D-1 | Prompt 层级 | 方案 A：面具在顶层 | cosplay 效果最强 |
| D-2 | 默认面具 | "我就是我"，不可删除 | 等同无面具，用户无感知 |
| D-3 | 子 agent 工具继承 | child ⊆ parent | 参考 OpenFang capability 模型 |
| D-4 | Batch spawn 失败策略 | 返回全部结果（含失败） | 主 agent 决定是否重试 |
| D-5 | 切换时是否注入消息 | 是，注入系统消息 | LLM 感知切换事件 |
| D-6 | 目录结构 | 支持嵌套子目录 | 灵活组织 |
| D-7 | Pipeline scope | 不做特殊原语，父 agent 自编排 | 参考 Hermes，system-v 再考虑 |
| D-8 | 子 agent 是否继承面具工具限制 | 是，child ⊆ parent | 安全工程（harness）核心 |
| D-9 | 切换时上下文管理 | 先 compress → 注入切换消息 → 注入面具 prompt | 干净的角色切换 |
| D-10 | 面具存储格式 | Markdown + frontmatter（非 TOML） | 可读性好，与现有 Soul 文件风格一致 |

---

## 6. Success Metrics

| 指标 | 目标 |
|------|------|
| 面具切换成功率 | > 99% |
| 切换后 LLM 正确识别角色 | 人工验证通过 |
| 子 agent 工具限制生效 | runtime 测试覆盖 |
| 预置面具可用性 | 4 个预置面具开箱即用 |

---

## 7. Dependencies

| 依赖 | 说明 |
|------|------|
| SOUL 系统 | 面具 prompt 拼接在 SOUL prompt 之上 |
| ContextBuilder | 修改 `build_system_prompt()` 支持面具注入 |
| SubagentManager | 修改子 agent 创建逻辑，支持工具收窄 |
| Config schema | 新增 `MaskConfig`、`ToolLimits` 类型 |
| GUI (agent-diva-gui) | Header 切换器、Settings 面板 |

---

## 8. Open Questions

> ~~全部已确认，见 Decisions Log。~~

---

## 9. References

| 文件 | 说明 |
|------|------|
| `soul-mechanism-analysis.md` | OpenClaw SOUL 生命周期分析（设计基础） |
| `soul-persona-gap-implementation-checklist.md` | SOUL 人格能力补齐清单 |
| `agent-diva-core/src/config/schema.rs` | SubagentToolsConfig 定义 |
| `.workspace/openfang/` | OpenFang capability 继承参考 |
