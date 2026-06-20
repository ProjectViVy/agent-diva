# Compaction 运维手册 (Runbook)

> 文档版本: 1.0 | 更新日期: 2026-06-07 | 关联 ADR-0010

## 概述

Context Compaction 是 agent-diva 的上下文压缩机制，当对话 session 的 token 估算接近 provider 上下文窗口限制时，自动（或手动）将早期消息压缩为摘要，释放上下文空间。

## 架构速览

```
TokenEstimator ──► ContextBudgetMonitor ──► ContextCompactor
     │                      │                       │
 chars→tokens          .check()               .compact()
                        budget→pct             history→summary
```

**三种触发方式:**
- **Auto**: 每次处理消息前检查 budget，超过阈值自动触发
- **Manual**: 用户发送 `/compact` 命令
- **Reactive**: provider 返回 `context_length_exceeded` 错误时紧急触发

---

## 1. 如何观察 Compaction 状态

### 1.1 日志关键字

```bash
# 查看 compaction 触发日志
grep "Compaction triggered" agent-diva.log

# 查看 compaction 完成日志（含消息数、摘要长度、质量分数）
grep "Compaction complete" agent-diva.log

# 查看质量评分详情
grep "Compaction attempt" agent-diva.log

# 查看 reactive compaction
grep "Context overflow detected" agent-diva.log
grep "Reactive compaction" agent-diva.log

# 查看 compaction 失败
grep "Compaction failed" agent-diva.log
```

### 1.2 Session JSON 状态

Session 文件中 compaction 相关字段:

```json
{
  "last_compacted": 290,
  "compaction_history": [
    {
      "schema_version": 1,
      "compact_id": "compact-20260607-143022-a1b2c3d4",
      "created_at": "2026-06-07T14:30:22Z",
      "trigger": "auto",
      "source_range": { "start_index": 0, "end_index": 200 },
      "kept_recent_count": 10,
      "pre_compact_message_count": 200,
      "pre_compact_estimated_tokens": 12500,
      "summary": "...",
      "quality_score": 0.85,
      "retry_count": 0
    }
  ]
}
```

**关键字段解读:**
- `last_compacted`: 已压缩到的消息索引，`get_history()` 只返回此索引之后的消息
- `compaction_history`: 压缩历史链（支持多次压缩）
- `trigger`: 触发方式 (`auto` / `manual` / `reactive`)
- `source_range`: 被压缩的消息范围 `[start, end)`
- `quality_score`: 摘要质量分 (0.0–1.0)，低于 0.6 会触发重试
- `retry_count`: 重试次数（0 = 首次成功）

### 1.3 Budget 压力比

日志中 `budget pressure` 字段显示当前上下文压力:

```
Compaction triggered — budget pressure 85.3% (12800 tokens used of ~15000 history budget)
```

- `< 80%`: 安全区
- `≥ 80%`: 触发 compaction 阈值
- `> 100%`: 已超出 budget（可能触发 reactive compaction）

---

## 2. 如何手动触发 Compaction

### 2.1 `/compact` 命令

在对话中发送:
```
/compact
```

agent-diva 会立即对当前 session 执行压缩，返回:
```
compact done — 150 messages compressed, ~9500 tokens saved
summary: 用户与助手讨论了项目架构设计...
```

### 2.2 适用场景
- 对话即将进入长任务，想提前释放上下文空间
- 感觉 agent 回复质量下降（可能上下文过长）
- 切换话题前清理历史上下文

---

## 3. 质量评分解读

### 3.1 评分维度

| 维度 | 权重 | 说明 |
|------|------|------|
| Length (长度) | 20% | 摘要至少 50 字符，200+ 字符满分 |
| Keyword Coverage (关键词覆盖) | 40% | 源消息关键词在摘要中的覆盖率，≥30% 合格 |
| Semantic Completeness (语义完整性) | 40% | 是否包含完整句子（句号/问号/感叹号） |

### 3.2 评分阈值

- **≥ 0.6**: 合格，采用该摘要
- **< 0.6**: 不合格，触发重试（最多 2 次重试，共 3 次尝试）
- 3 次均不合格: 采用得分最高的那次摘要

### 3.3 常见质量问题

| 问题 | 日志关键字 | 原因 | 处理 |
|------|-----------|------|------|
| 摘要过短 | `摘要过短` | LLM 返回了过于简略的摘要 | 自动重试，通常第 2 次改善 |
| 关键词覆盖低 | `关键词覆盖率过低` | 摘要遗漏了重要技术术语 | 自动重试 |
| 缺少完整句子 | `缺少完整句子` | LLM 返回了列表或碎片 | 自动重试 |

---

## 4. 如何调整 BudgetConfig

### 4.1 配置项

在 `config.yaml` 的 `tools.compaction` 部分:

```yaml
tools:
  compaction:
    max_tokens: 180000          # 最大上下文 token 数
    system_budget_ratio: 0.15   # system prompt 预留比例
    compact_threshold_ratio: 0.80  # 触发压缩的阈值比例
    keep_recent_count: 10       # 保留最近 N 条消息不压缩
```

### 4.2 调参建议

| 场景 | 调整 | 效果 |
|------|------|------|
| 频繁触发 compaction | 增大 `max_tokens` | 减少触发频率，但消耗更多 API tokens |
| 对话连贯性差 | 增大 `keep_recent_count` | 保留更多近期上下文，但压缩效果减弱 |
| 想更早触发压缩 | 降低 `compact_threshold_ratio` | 更早压缩，避免 reactive compaction |
| 使用小上下文模型 | 降低 `max_tokens` | 适配 8K/16K 上下文模型 |

### 4.3 推荐配置

| Provider | max_tokens | 说明 |
|----------|-----------|------|
| DeepSeek V3 (128K) | 180000 | 默认配置，留有余量 |
| GPT-4o (128K) | 180000 | 同上 |
| Claude Sonnet (200K) | 250000 | 可适当增大 |
| 小模型 (8K) | 8000 | 需降低 keep_recent_count |

---

## 5. 故障排查

### 5.1 Compaction 不触发

**症状**: 对话很长但没有压缩日志

**排查步骤**:
1. 检查 `max_tokens` 是否设置过大
2. 检查 `compact_threshold_ratio` 是否过高
3. 查看日志中 `budget pressure` 值
4. 确认 `get_history()` 返回的消息数（受 `keep_recent_count` 影响）

### 5.2 Compaction 失败

**症状**: 日志显示 `Compaction failed`

**排查步骤**:
1. 检查 provider API 是否可用
2. 检查 API key 是否有效
3. 查看具体错误信息（网络超时、rate limit 等）
4. Compaction 失败不会阻断对话，agent 会继续使用未压缩的上下文

### 5.3 Reactive Compaction 频繁触发

**症状**: 经常看到 `Context overflow detected`

**原因**: `max_tokens` 设置过大，导致 auto compaction 没有及时触发，直到 provider 返回错误

**解决**: 降低 `max_tokens` 或降低 `compact_threshold_ratio`，让 auto compaction 更早介入

### 5.4 摘要质量差

**症状**: agent 在压缩后"忘记"了重要上下文

**排查步骤**:
1. 检查 `quality_score`，低于 0.7 可能有质量问题
2. 查看 `retry_count`，如果总是重试到上限，可能是 LLM 对 compaction prompt 不友好
3. 考虑增大 `keep_recent_count` 保留更多近期消息

---

## 6. 性能指标

### 6.1 Token 节省

典型场景下的 token 节省比例:

| 消息数 | 压缩前 tokens | 压缩后 tokens | 节省比例 |
|--------|-------------|-------------|---------|
| 100 | ~6,000 | ~600 | ~90% |
| 200 | ~12,000 | ~800 | ~93% |
| 300 (3 轮压缩) | ~18,000 | ~1,200 | ~93% |

### 6.2 压缩耗时

- 单次 compaction: 2-10 秒（取决于 LLM provider 响应速度）
- 含重试: 最多 30 秒（3 次尝试 × 10 秒）
- 对用户体感影响: compaction 在消息处理前同步执行，会增加首次响应延迟

### 6.3 重试统计

- 首次成功率: ~85%（quality_score ≥ 0.6）
- 重试后成功率: ~98%
- 3 次均失败: <2%（采用最高分摘要）

---

## 7. 多次压缩链 (Multi-Compaction Chain)

### 7.1 工作原理

当 session 持续增长，会触发多次压缩。每次压缩的范围是 `[last_compacted .. new_end]`，形成连续链:

```
第 1 次: [0 .. 200]   → summary_1
第 2 次: [200 .. 280] → summary_2 (融合 summary_1)
第 3 次: [280 .. 290] → summary_3 (融合 summary_1 + summary_2)
```

### 7.2 层级摘要

每次压缩时，之前的摘要会作为上下文传给 LLM，生成融合摘要。这确保了即使经过多次压缩，关键信息仍能保留。

### 7.3 build_messages 注入

LLM 看到的上下文结构:
```
[system prompt]
[## Context Compaction Boundary]
[compacted context start]
[summary_1]
[compacted context end]
[## Context Compaction #2]
[compacted context start]
[summary_2]
[compacted context end]
[## Context Compaction #3]
[compacted context start]
[summary_3]
[compacted context end]
[recent messages...]
[current user message]
```

---

## 8. 向后兼容

### 8.1 旧格式迁移

旧版 session 文件使用 `compaction: Option<CompactSummary>`（单个对象）。新版使用 `compaction_history: Vec<CompactSummary>`（数组）。反序列化时自动兼容:

- 旧格式 `"compaction": { ... }` → 自动转为 1 元素 Vec
- 新格式 `"compaction_history": [ ... ]` → 直接使用

### 8.2 零 downtime 升级

无需迁移脚本，旧 session 文件在首次加载时自动转换格式。
