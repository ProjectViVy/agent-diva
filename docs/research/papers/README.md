# 论文参考集

本目录包含 agent-diva 项目研究相关的学术论文，用于支持项目的理论设计和架构决策。

## 论文列表

| arXiv ID | 标题 | 文件 | 相关性 |
|----------|------|------|--------|
| 2304.03442 | Generative Agents (Park et al., Stanford 2023) | `2304_03442.{md,pdf}` | T1 直接相关 - diva 自主活动的理论蓝图 |
| 2305.02750 | A Survey on Proactive Dialogue Systems (Deng et al., IJCAI 2023) | `2305_02750.{md,pdf}` | T1 直接相关 - Initiative/Anticipation/Planning 三要素框架 |
| 2410.12361 | Proactive Agent (Lu et al., THUNLP, ICLR 2025) | `2410_12361.{md,pdf}` | T1 直接相关 - 主动出击的形式化定义 + 评测基准 |
| 2412.14352 | A Survey on LLM Inference-Time Self-Improvement (Dong et al., 2024) | `2412_14352.{md,pdf}` | T1 相关 - 自我升级的分类与挑战 |
| 2210.03629 | ReAct (Yao et al., 2022) | `2210_03629.{md,pdf}` | T2 相关架构 - Reasoning + Acting 交错范式 |
| 2303.11366 | Reflexion (Shinn et al., NeurIPS 2023) | `2303_11366.{md,pdf}` | T2 相关架构 - 言语强化学习，self-improve 思路 |
| 2212.08073 | Constitutional AI (Bai et al., Anthropic 2022) | `2212_08073.{md,pdf}` | T2 相关架构 - 原则驱动的 AI 行为约束 |
| 2604.17091 | GenericAgent (Liang/Han et al., A3 Lab 2026) | `genericagent.md` | R1c 直接相关 - 自我进化叙事；对照源码见 R1 `ga-autonomy-origin.md` |

## 解析说明

- **解析工具**: [Nebutra/MinerU-Skill](https://github.com/Nebutra/MinerU-Skill) `scripts/mineru.py`
- **格式**: 每篇论文包含 `.pdf` 原始文件和 `.md` 解析后的 Markdown 版本
- **备份**: 本目录是 `/Users/mastwet/Desktop/morediva/papers/` 的备份副本

## 使用说明

这些论文为 agent-diva 项目的以下方面提供理论支持：
1. **自主活动设计**: Generative Agents 和 Proactive Agent 论文
2. **对话系统框架**: Proactive Dialogue Systems 论文
3. **自我改进机制**: Self-Improvement Survey 和 Reflexion 论文
4. **代理架构**: ReAct 和 Constitutional AI 论文
5. **GenericAgent 自我进化**: `genericagent.md`（arXiv:2604.17091）；以本地 GA 源码为准，论文更满