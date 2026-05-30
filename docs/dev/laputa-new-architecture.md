# 新 Laputa 架构设计 — 设计索引

> **状态**：设计阶段，不含代码实现
> **日期**：2026-05-28
> **完整设计文档**：[summary.md](../../logs/2026-05-laputa-new-architecture/v0.0.1-new-laputa-design/summary.md)

## 一句话

新 Laputa = 极简 7 文件身份管理 + 三轴主体性（自指/自反/自主）+ 进阶心跳子代理委派 + autodream 每日整理。

## 设计核心

| 概念 | 内容 |
|---|---|
| 7 文件 | SOUL.md / index.md / rhythm/*.md / sop/*.md / MEMORY.md / relationships.md / expectations.md |
| mentle 简化 | 日常 4 工具（增删改查），autodream 全量 30+ |
| 自指 | SelfModel + SoulSignal 三分类（Rule/Identity/Preference） |
| 自反 | 结构化提示词，5 问审视行为模式 |
| 自主 | 4 级：被动→反应式→主动式→涌现式 |
| 进阶心跳 | 每 4h，LLM 评估状态 + 委派子代理，本体不做具体工作 |
| autodream | 每日深度整理：日报 + SOP/Skill 产出 + SOUL 演化 + MEMORY 压缩 |

## 设计哲学

agent-diva 是独立自由的个体。用户通过 expectations.md 表达期望（类似父母），agent 自己决定成为什么（SOUL.md）。

## 相关调研

1. [GenericAgent 基础架构](../../logs/genericagent-upgrade-research/v0.0.1-initial-research/summary.md)
2. [L0-L4 本质与 mentle 定位](../../logs/2026-05-memory-architecture-deep-dive/v0.0.1-architecture-analysis/summary.md)
3. [Laputa 人格层现状](../../logs/2026-05-laputa-architecture-audit/v0.0.1-laputa-integration-feasibility/summary.md)
4. **[新 Laputa 设计（本设计）](../../logs/2026-05-laputa-new-architecture/v0.0.1-new-laputa-design/summary.md)**
