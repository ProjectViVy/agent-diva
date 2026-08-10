# 技术研究报告

本目录包含 agent-diva 项目的技术研究报告，涵盖架构设计、技术对比、集成方案等内容。

## 报告列表

### 后台任务队列管线设计研究报告.md
- **研究范围**: Alife (C#) / Hermes (Python) / Diva (Rust)
- **生成时间**: 2026-06-19
- **分析视角**: Harness Engineering — 后台任务/子进程/队列管线
- **核心发现**: 
  - Alife: 最简设计，只有 Poke 消息缓存队列 + 模块级定时轮询
  - Hermes: 最成熟，线程池调度 + 进程注册表 + 双层限流通知
  - Diva: 基础设施最好，但缺通用后台任务队列/管线抽象

### Alife 相关研究
- `alife-function-skill-memory.md` - Alife 功能、技能、内存研究
- `alife-harness-gap-inventory.md` - Alife Harness 差距清单
- `alife-harness-overview.md` - Alife Harness 概述
- `alife-plugin-module-di.md` - Alife 插件模块依赖注入
- `alife-proactive-selfupgrade.md` - Alife 主动自我升级
- `alife-user-presence-scan.md` - Alife 用户存在扫描
- `alife-vs-diva-autonomous-flow.md` - Alife vs Diva 自主流程对比

### Hermes 相关研究
- `hermes-cronjob-tool.md` - Hermes 定时任务工具
- `hermes-harness-overview.md` - Hermes Harness 概述

### 集成与对比研究
- `diva-alife-integration-plan.md` - Diva Alife 集成计划
- `harness-engineering-three-way-comparison.md` - Harness Engineering 三方对比
- `harness-engineering-three-way-detailed-checklist.md` - Harness Engineering 三方详细清单
- `workspace-capability-matrix.md` - .workspace 13 项目 × agent-diva 综合能力矩阵（2026-07-03；15 维度合成）
- `workspace-hooks-comparison.md` - .workspace Agent Hooks 实现横向对比（2026-07-03）
- `workspace-subagent-comparison.md` - .workspace Sub-Agent 实现横向对比（2026-07-03；§7.1 含 `feature-swarm-humanlike` 分支预览）

### 沙箱 / 审批 / HITL
- `approval-model-claude-code-vs-agent-diva.md` - Claude Code 审批模型对照调研（2026-08-05）
- `sandbox-hitl-approval-policy-proposal.md` - 沙箱审批策略 + HITL 完善提案（生产路径复核与 P0–P2 蓝图；2026-08-05 归档）
- `ask-user-clarify-hitl-proposal.md` - **对话询问** Clarify/Ask-User HITL 缺口研究与提案（与 M3 审批 HITL 分轨；2026-08-05 归档）

### Laputa / BML / Garden 架构
- `laputa-garden-cognitive-sync-2026-08/gap-and-migration-proposal.md` - 桌面 Garden 认知分区回迁权威提案（ADR-0002/0004/0008；2026-08-07 冻结）
- `bml-layer-extraction-2026-08/bml-layer-extraction-research.md` - **BML 存储层独立化调研**：耦合盘点（H1–H6 验证）、Garden 三 module 同构对照、抽层方案 A/B/C/D 对比与决策建议（2026-08-08，待拍板）

### 上下文管理 / Harness Context
- `context-management-enhancement-2026-08/` - **上下文管理增强**（2026-08-10）
  - **优先读** `claude-code-prompt-cache-alignment.md` — Claude Code 导向 Prompt Cache 对齐规格（C1 / P0-1..P0-5）
  - `c0-baseline-and-architecture-decisions.md` — C0 基线与 ADR-CTX-0..5 总论

## 使用说明

这些研究报告为 agent-diva 项目的架构设计和技术选型提供参考，可用于：
- 了解不同框架的技术特点和优劣
- 参考集成方案和最佳实践
- 指导技术决策和架构演进