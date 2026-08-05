# OpenHarness Research Summary

## 1. 任务背景
用户指定对 `.workspace/OpenHarness` 进行深度调研，对比 `Claude Code` 与 `Agent-Diva` 的 Harness 架构与机制，提炼对 Agent-Diva 的核心启发与演进提案，且不修改任何代码（“不写代码。仅提案”）。

## 2. 核心调研发现
1. **OpenHarness `oh --dry-run` 离线预检**：能在不调用模型/不执行工具的前提下，预检 Auth、MCP、Prompt-Tools 命中并给出 `ready/warning/blocked` 与 `next_actions`。
2. **Workflow Profile 统一凭据层**：支持 Workflow 下多 Profile 秘钥隔离，解决多个兼容 API 后端共享 Key 的覆盖问题。
3. **HookEvent 生命周期管道**：定义 10+ 显式 HookEvent 枚举，支持生命周期插件拦截。
4. **`ohmo` 个人 Agent 应用**：轻量个人 Agent 根工作区与多 IM Channel 集成。

## 3. 交付物
- 调研报告：`C:\Users\Administrator\Desktop\morediva\openharness-claude-code-diva-research.md`
- 迭代记录：`C:\Users\Administrator\Desktop\morediva\agent-diva\docs\logs\2026-08-05-openharness-research/v0.1.0-openharness-proposal/`
- 待办更新：`C:\Users\Administrator\Desktop\morediva\agent-diva\TODOLIST.md`
