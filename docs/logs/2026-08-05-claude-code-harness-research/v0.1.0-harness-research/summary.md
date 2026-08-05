# Claude Code vs Agent-Diva Harness Research Summary

## 1. 任务背景与目标
用户要求对 `morediva/.workspace/claude-code` 与 `morediva/agent-diva` 进行全面性调研，对比基础 Harness（Agent 调度与控制框架）能力的差距，不修改任何代码，仅进行架构与机制调研，并将文档保存在 `morediva` 目录。

## 2. 核心调研成果
在 `morediva/claude-code-vs-agent-diva-harness-research.md` 中产出了完整的对比与差距分析报告，涵盖以下 6 大 Harness 维度：

1. **Context & Budget Harness（上下文预算、压缩与 Prompt Caching 治理）**
   - Claude Code 拥有 API Prompt Cache 结构匹配、TF-IDF 工具按需索引加载（`EXPERIMENTAL_SEARCH_EXTRA_TOOLS`）、Micro-compaction 与 Tombstone 剪裁。
   - Agent-Diva 的上下文拼接相对静态，全量挂载工具 Schema。

2. **Tooling & Engineering Harness（工具演进与代码感知）**
   - Claude Code 集成了 LSP 语言服务协议（`LSPTool`）、Plan Mode 物理只读拦截状态机、NAPI 彩色终端 Diff 渲染。
   - Agent-Diva 依靠文本检索与行号匹配，Plan Mode 仅为提示词指导。

3. **Subagent & Parallel Harness（多 Agent 隔离与任务调度）**
   - Claude Code 实现了基于 Git Worktree 的子 Agent 工作区物理隔离，以及 CLI 原生后台任务 Session 管理（`ps`/`logs`/`attach`）。
   - Agent-Diva 共享物理工作区，多子 Agent 并发写易发生冲突。

4. **Remote & Cloud Harness（端云桥接与协同）**
   - Claude Code 自带 Remote Control Server (RCS)、ACP 协议 Web UI 与 Cloudflare Worker HTML Artifact 托管。
   - Agent-Diva 侧重于本地 HTTP 控制面与桌面 Tauri GUI。

5. **Provider & Stream Harness（API 流式适配与成本跟踪）**
   - Claude Code 采用流适配器归一化不同 API，原生支持 DeepSeek 思考链 Token 解析与按 Turn 成本核算。
   - Agent-Diva 拥有严格的 Provider Model-ID 安全防护规则。

6. **Sandbox & Safety Harness（沙箱隔离与安全控制）**
   - Agent-Diva 拥有 Rust 原生沙箱与独创的 LLM Guardian 安全评估进程，在零信任架构上占据优势。

## 3. 产出文档列表
- 主调研报告：`C:\Users\Administrator\Desktop\morediva\claude-code-vs-agent-diva-harness-research.md`
- 待办更新：`C:\Users\Administrator\Desktop\morediva\agent-diva\TODOLIST.md`
- 迭代日志：`C:\Users\Administrator\Desktop\morediva\agent-diva\docs\logs\2026-08-05-claude-code-harness-research/v0.1.0-harness-research/`
