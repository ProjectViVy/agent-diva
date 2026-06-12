# BMad 信息集 — 报表系统 & Session 历史检索 PRD 输入源

> 生成时间: 2026-06-08
> 来源: 用户决策 + 代码调研 + 项目参考
> 状态: PRD 输入源（非实现）

---

## 决策记录 (Decision Log)

| # | 问题 | 用户决策 | 备注 |
|---|------|---------|------|
| 1 | Report 数据来源 | **A — 基于 session 历史自动汇总** | 第一阶段仅实现 A |
| 2 | Report 触发机制 | **两者皆可** | 支持定时自动生成 + 用户手动触发 |
| 3 | SOP/Skill/Memory 固化参考 | **参考 Hermes + GenericAgent** | 参考项目见 `.workspace/` 目录 |
| 4 | Report 存储介质 | **独立 Markdown 文件** | 非 JSONL/SQLite |
| 5 | 搜索范围 | **所有历史 session** | 不限于当前 session |
| 6 | 搜索方式 | **Agent 智能搜索** | 作为一个 session 任务来执行 |
| 7 | 结果呈现 | **不一定可视化** | 不要求 GUI 展示搜索结果 |
| 8 | Session 原子写入修复 | **需要修复** | cherry-pick main 的 `write_session_atomically` |
| 9 | Session 搜索方案 | **A — 内存遍历 + 正则** | 短期方案，大数据量时性能差 |

---

## 1. 功能概述

### 1.1 报表系统 (Notebook)

**目标**: 让 Diva 能够基于 session 历史自动生成日报、周报、月报，并支持用户查看和管理。

**核心功能**:
- 自动/手动生成日报、周报、月报
- 基于 session 历史自动汇总（LLM 生成摘要）
- Report 以独立 Markdown 文件形式存储
- 支持将 Report 固化为 SOP、Skill 或更新长期记忆

**参考项目**:
- Hermes (`.workspace/hermes-learning/`)
- GenericAgent (`.workspace/` 下相关项目)

### 1.2 Session 历史检索

**目标**: 让 Diva 能够搜索并回顾所有历史对话记录。

**核心功能**:
- 搜索范围：所有历史 session
- 搜索方式：Agent 智能搜索（作为一个 session 任务执行）
- 实现方案：内存遍历 + 正则匹配（短期方案）
- 结果呈现：不要求可视化，可通过 API/命令行返回

---

## 2. 技术现状

### 2.1 报表系统

| 组件 | 状态 | 文件位置 |
|------|------|---------|
| GUI (NotebookView.vue) | ✅ 完整 | `agent-diva-gui/src/components/NotebookView.vue` |
| Tauri 后端命令 | ❌ 缺失 | 需新增 `get_notebook_reports` 等命令 |
| Report 数据模型 | ❌ 不存在 | 需定义 NotebookReport 结构 |
| Report 生成逻辑 | ❌ 不存在 | 需基于 session 历史调用 LLM 生成 |
| Report 存储 | ❌ 不存在 | 需实现独立 Markdown 文件存储 |

### 2.2 Session 持久化

| 项目 | 主分支 (main) | Pro 分支 |
|------|--------------|----------|
| 存储格式 | JSONL | JSONL (相同) |
| 原子写入 | ✅ 有 | ❌ 缺失（需修复） |
| 备份机制 | ✅ 有 | ❌ 缺失 |
| 已知 Bug | 27 个 | 比 main 更脆弱 |

**修复项**:
- cherry-pick main 的 `write_session_atomically` 到 pro
- 添加 `.jsonl.bak` 备份机制

### 2.3 Session 历史检索

| 能力 | 状态 |
|------|------|
| 按 session key 精确读取 | ✅ |
| 列出所有 session | ✅ |
| 按内容关键词搜索 | ❌（需实现） |
| 按时间范围过滤 | ❌（需实现） |
| 语义搜索 | ❌（未来考虑） |

---

## 3. 用户故事

### US-1: 自动/手动生成报表

**作为** 用户，
**我希望** Diva 能基于 session 历史自动生成日报/周报/月报，
**以便** 我快速回顾与 Diva 的互动情况。

**验收标准**:
- [ ] 支持定时自动生成（cron）
- [ ] 支持用户手动触发生成
- [ ] Report 内容基于 session 历史自动汇总（LLM 生成）
- [ ] Report 以 Markdown 文件形式存储

### US-2: 查看报表

**作为** 用户，
**我希望** 在 GUI 中查看生成的日报/周报/月报，
**以便** 了解 Diva 的总结和回顾。

**验收标准**:
- [ ] GUI 已就绪（NotebookView.vue）
- [ ] 后端 `get_notebook_reports` 命令返回 Report 列表
- [ ] 支持按 daily/weekly/monthly 过滤

### US-3: 固化 Report

**作为** 用户，
**我希望** 将 Report 固化为 SOP、Skill 或更新长期记忆，
**以便** 沉淀知识和经验。

**验收标准**:
- [ ] 支持"固化为 SOP"（参考 Hermes/GenericAgent）
- [ ] 支持"固化为 Skill"（参考 Hermes/GenericAgent）
- [ ] 支持"更新长期记忆"（参考 Hermes/GenericAgent）

### US-4: 搜索历史对话

**作为** 用户，
**我希望** 让 Diva 搜索所有历史对话记录，
**以便** 回顾过去的讨论内容。

**验收标准**:
- [ ] 搜索范围覆盖所有历史 session
- [ ] 搜索方式：Agent 智能搜索（作为 session 任务执行）
- [ ] 实现方案：内存遍历 + 正则匹配
- [ ] 不要求可视化展示（可通过 API/命令行返回）

---

## 4. 技术方案

### 4.1 Report 生成

**数据来源**: Session 历史（JSONL 文件）
**生成方式**: LLM 自动汇总
**触发机制**:
- 定时触发：cron 任务（每日/每周/每月）
- 手动触发：用户点击 GUI 中的"生成"按钮

**Report 结构**:
```typescript
interface NotebookReport {
  id: string;           // 唯一标识
  date: string;         // ISO 日期
  title: string;        // 标题
  summary: string;      // 摘要
  content: string;      // 完整 Markdown 内容
  period: 'daily' | 'weekly' | 'monthly';
  source_sessions: string[];  // 来源 session keys
}
```

**存储方案**:
- 格式：独立 Markdown 文件
- 路径：`{workspace}/reports/{period}/{YYYY-MM-DD}.md`
- 每个 Report 一个文件，便于版本管理和人工查看

### 4.2 Report 固化

**参考项目**: Hermes + GenericAgent (`.workspace/`)

| 固化类型 | 目标 | 参考实现 |
|---------|------|---------|
| SOP | 生成标准操作流程文档 | 参考 Hermes skill 格式 |
| Skill | 创建可复用的 skill 文件 | 参考 GenericAgent skill 格式 |
| Memory | 更新长期记忆存储 | 参考 Hermes memory provider |

### 4.3 Session 历史检索

**搜索范围**: 所有历史 session
**搜索方式**: Agent 智能搜索（作为 session 任务执行）
**实现方案**: 内存遍历 + 正则匹配

**算法**:
1. 加载所有 session 文件到内存
2. 对每个 session 的 messages 进行遍历
3. 使用正则匹配搜索关键词
4. 返回匹配的 message 列表（含 session key、timestamp、content）

**优化方向**（未来）:
- 中期：SQLite FTS5
- 长期：嵌入式向量检索

---

## 5. 依赖项

| 依赖 | 状态 | 说明 |
|------|------|------|
| Session 持久化修复 | 🔧 需修复 | cherry-pick `write_session_atomically` |
| LLM 调用能力 | ✅ 已有 | 用于 Report 生成 |
| Markdown 渲染 | ✅ 已有 | GUI 已支持 |
| Report 存储目录 | ❌ 需创建 | `{workspace}/reports/` |

---

## 6. 风险与注意事项

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| Session 数据量大时搜索慢 | 高 | 短期接受，中期迁移到 SQLite |
| Report 生成失败 | 中 | 添加错误处理和重试机制 |
| Report 固化格式不兼容 | 中 | 参考 Hermes/GenericAgent 标准格式 |
| Session 原子写入缺失导致数据丢失 | 高 | 优先修复 `write_session_atomically` |

---

## 7. 参考文件

- `docs/research/report-session-research-info-set.md` — 完整调研报告
- `agent-diva-gui/src/components/NotebookView.vue` — GUI 实现
- `agent-diva-core/src/session/store.rs` — Session 数据结构
- `agent-diva-core/src/session/manager.rs` — Session 管理器
- `.workspace/hermes-learning/` — Hermes 参考项目
- `.workspace/` — GenericAgent 参考项目
