# Archive 治理说明

> 本目录为 `docs/dev/archive(old-docs-dont-read-me)` 的治理后版本。
> 治理日期：2026-06-13

---

## 治理结果

| 文件/目录 | 说明 |
|-----------|------|
| `DONE.md` | 已完成的决策、closeout、合并记录索引 |
| `PENDING-DECISIONS.md` | 14 项待拍板决策，含现状/选项/建议/阻塞点 |
| `RESEARCH-精华/` | 5 篇压缩后的调研精华 |
| `README.md` | 本文件 |
| *(其他子目录)* | 原始文档仍保留，按主题分类 |

---

## 快速导航

### 想了解「已完成什么」→ 看 DONE.md
- 分支合并记录（main→pro）
- Closeout 卡片（MAIN-CLOSE-01~05）
- 已完成的调研与决策
- 已废弃的旧架构声明

### 想了解「还有什么没决定」→ 看 PENDING-DECISIONS.md
- 14 项待决策，按优先级排序
- 每项含：现状、选项、建议、阻塞点

### 想了解「调研结论」→ 看 RESEARCH-精华/
- AwesomeAgents 7 项目对比
- EvoMap/GEP 接入策略
- 沙箱安全审计摘要
- Bug 修复经验
- Main→Pro 合并策略

### 想查原始全文 → 进对应子目录
- `agent-diva-main/` — main 分支历史文档
- `agent-diva-pro-legacy/` — pro 分支 legacy 文档
- `awesomeagents/` — 7 项目调研原始文档
- `architecture-reports/` — 架构报告原始文档
- `mentle-integration/` — Mentle sprint 原始文档
- `sandbox-audit/` — 沙箱审计原始文档
- ...（其他专题目录）

---

## 已清理内容

以下文档已被移除（无保留价值）：

- `_bmad-output-legacy*` — BMad 自动生成输出（6 项）
- `archive/` 根目录 — `memory-evolution` + `nanobot-sync` 的重复副本
- `agent-diva-main/docs/dev/archive(old-docs-dont-read-me)/archive/` — 嵌套重复副本

---

## 与当前主架构冲突的文档声明

以下旧架构/决策与当前 `agent-diva-pro` 主架构冲突，**不应再引用**：

1. **外部 gateway 进程模式** — 已替换为嵌入式模式
2. **`agent-diva-nano` 内嵌方案** — 已外化为独立仓库
3. **前端/产品 UI 规划（main closeout 排除项）** — 已移至 pro 分支独立演进
4. **Mentle 全量集成计划（S1-S6）** — 已完成，S7+ 在 pro 分支继续
5. **旧版 provider 配置链** — 已重构为统一 Provider trait + LiteLLM 兼容
6. **旧版 skill 加载机制** — 已迁移至 SkillLoader + 缓存体系
