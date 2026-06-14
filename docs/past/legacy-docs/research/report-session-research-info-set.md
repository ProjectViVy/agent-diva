# Agent-Diva Pro 报表系统 & Session 历史检索 — 调研信息集

> 生成时间: 2026-06-08
> 调研范围: agent-diva-pro (当前分支) + agent-diva (main 分支)
> 调研方法: 代码审计 + 文档分析 + 子代理并行调研

---

## 1. 需求背景

用户要求完善 agent-diva-pro 分支的**日报、周报、月报系统 PRD**，目前 GUI 已完整，需要补实际功能。同时新增需求：**让 Diva 能够自己寻找自己的历史对话记录**。主分支已完成 session 持久化（JSONL 格式），需要调研现状并收集需求。

---

## 2. 报表系统现状调研

### 2.1 UI 层 — 已完成

**文件**: `agent-diva-gui/src/components/NotebookView.vue` (725 行)

| 组件 | 状态 | 说明 |
|------|------|------|
| 双栏布局 | ✅ | 左侧列表 + 右侧详情 |
| 周期切换 | ✅ | daily/weekly/monthly 三标签 |
| Markdown 渲染 | ✅ | 含代码高亮 (highlight.js) |
| 骨架屏/空态/错误态 | ✅ | 完整的 loading 和 empty 状态 |
| 底部操作栏 | ✅ | 固化 SOP、固化 Skill、更新长期记忆 |
| 轮询刷新 | ✅ | 60s 自动 poll |

**关键代码**:
```typescript
// NotebookView.vue 第 86-94 行
async function fetchReports() {
  if (isTauri()) {
    reports.value = await invoke<NotebookReport[]>('get_notebook_reports', {
      period: activePeriod.value,
    });
  } else {
    reports.value = []; // Browser preview: mock data
  }
}
```

### 2.2 后端 — 完全缺失

| Tauri 命令 | 调用位置 | 实现状态 |
|-----------|---------|---------|
| `get_notebook_reports` | NotebookView.vue:88 | ❌ 未实现 |
| `solidify_report_as_sop` | NotebookView.vue:137 | ❌ 未实现 |
| `solidify_report_as_skill` | NotebookView.vue:157 | ❌ 未实现 |
| `update_memory_from_report` | NotebookView.vue:179 | ❌ 未实现 |

**结论**: GUI 是完整的"空壳"，所有数据获取和持久化逻辑均未实现。

### 2.3 国际化

**文件**: `agent-diva-gui/src/locales/zh.ts` / `en.ts`

已定义 notebook 相关文案:
- `notebook.title`: 记事本 / Notebook
- `notebook.periodDaily`: 日报 / Daily
- `notebook.periodWeekly`: 周报 / Weekly  
- `notebook.periodMonthly`: 月报 / Monthly
- `notebook.solidifySop`: 固化为 SOP
- `notebook.solidifySkill`: 固化为 Skill
- `notebook.updateMemory`: 更新长期记忆

---

## 3. Session 持久化与历史检索调研

### 3.1 存储格式

**主分支 (agent-diva/main)**:
- 格式: JSONL (每行一个 JSON 对象)
- 路径: `{workspace}/sessions/{safe_key}.jsonl`
- 结构: metadata 行 + message 行数组
- 原子写入: ✅ 有 `write_session_atomically` (tmp → rename)
- 备份机制: ✅ `.jsonl.bak`

**Pro 分支 (agent-diva-pro)**:
- 格式: 相同 JSONL
- 原子写入: ❌ 使用 `std::fs::write` 直接覆盖（非原子）
- 备份机制: ❌ 无

### 3.2 Session 数据结构

**文件**: `agent-diva-core/src/session/store.rs`

```rust
pub struct Session {
    pub key: String,              // e.g. "gui:chat-xxx" or "telegram:12345"
    pub messages: Vec<ChatMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
    pub last_consolidated: usize,  // memory consolidation 边界
}

pub struct ChatMessage {
    pub role: String,              // user | assistant | system | tool
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<Vec<serde_json::Value>>,
    pub name: Option<String>,
    pub reasoning_content: Option<String>,
    pub thinking_blocks: Option<Vec<serde_json::Value>>,
}
```

### 3.3 现有 API 能力

**主分支 HTTP API** (`agent-diva-manager/src/server.rs`):

| 端点 | 方法 | 功能 |
|------|------|------|
| `/api/sessions` | GET | list_sessions — 列出所有 session |
| `/api/sessions/:id` | GET | get_session_history — 获取单个 session 完整历史 |
| `/api/sessions/:id` | DELETE | delete_session — 删除 session |

**Pro 分支 Tauri 命令**:
- 目前只有基础的 session 管理命令（create/delete/clear）
- 没有 `search_sessions` 或 `query_history` 类命令

### 3.4 已知问题 (来自 session-history-storage-research.md)

**主分支已识别的 27 个 Bug**:
- **P0 (9个)**: 用户消息 turn 完成前不写入、consolidation 在 save 前执行、JSONL 非原子覆盖写、save 失败无重试、GUI 缓存优先无失效、sendMessage 后不更新缓存等
- **P1 (10个)**: load I/O error → 空 session、JSONL 静默丢弃不可解析行、list_sessions key 编码不可逆等
- **P2 (8个)**: get_history 返回孤立 tool 消息、tool 结果截断到 500 字符等

**Pro 分支额外问题**:
- 原子写入缺失（比主分支更差）
- `get_or_load` 返回 `Option` 而非 `Result`，错误处理更弱

---

## 4. "Diva 寻找历史对话"需求分析

### 4.1 当前能力

| 能力 | 状态 | 限制 |
|------|------|------|
| 按 session key 读取完整历史 | ✅ | 需要知道准确的 session key |
| 列出所有 session | ✅ | 只有基础 metadata（created_at, updated_at） |
| 按内容搜索 message | ❌ | 无全文检索 |
| 按时间范围过滤 | ❌ | 无时间查询 API |
| 按关键词/语义搜索 | ❌ | 无搜索引擎集成 |

### 4.2 实现路径选项

| 方案 | 复杂度 | 优点 | 缺点 |
|------|--------|------|------|
| A. 内存遍历 + 简单匹配 | 低 | 快速实现 | 大数据量时性能差 |
| B. JSONL 全文扫描 | 中 | 无需额外依赖 | IO 密集，慢 |
| C. SQLite FTS5 | 中 | 标准全文检索 | 需要 schema 迁移 |
| D. 嵌入式向量检索 | 高 | 语义搜索 | 需要 embedding 模型 |

---

## 5. 关键发现总结

### 5.1 报表系统

```
现状: GUI 100% 完成，后端 0%
阻塞点: 
  1. 没有 report 数据模型和存储
  2. 没有 report 生成逻辑（何时/如何生成日报/周报/月报）
  3. 没有 SOP/Skill/Memory 的固化接口实现
```

### 5.2 Session 持久化

```
现状: JSONL 文件存储，基础读写可用
阻塞点:
  1. Pro 分支缺少原子写入（比 main 更脆弱）
  2. 没有内容搜索能力（只能按 key 读取）
  3. 27 个已知 bug 中部分影响数据可靠性
```

### 5.3 历史对话检索

```
现状: 无
技术选项:
  - 短期: 内存遍历 + 正则匹配（快速实现）
  - 中期: SQLite FTS5（标准方案）
  - 长期: 向量检索（语义搜索）
```

---

## 6. 待澄清问题（Open Questions）

### 6.1 报表系统

1. **Report 数据来源**: 日报/周报/月报的数据从哪里来？
   - 选项 A: 基于 session 历史自动汇总（LLM 生成摘要）
   - 选项 B: 基于用户手动输入/标记的内容
   - 选项 C: 基于系统事件日志（工具调用、错误、完成度等）
   - 选项 D: 混合模式

2. **Report 触发机制**: 
   - 定时自动生成（cron）还是用户手动触发？
   - 如果是自动，时间规则是什么？

3. **SOP/Skill/Memory 固化**:
   - "固化为 SOP" 的具体含义是什么？生成文件？更新配置？
   - "固化为 Skill" 是创建新的 skill 文件吗？格式是什么？
   - "更新长期记忆" 是写入哪个 memory provider？

4. **Report 存储**:
   - 存 JSONL？SQLite？独立文件？
   - 是否需要版本历史？

### 6.2 历史对话检索

5. **搜索范围**: 
   - 只搜索当前 session？还是所有历史 session？
   - 是否跨 channel 搜索（gui + telegram + discord 等）？

6. **搜索方式**:
   - 关键词匹配？正则？
   - 是否需要语义搜索（理解含义而非字面匹配）？
   - 是否需要时间范围过滤？

7. **结果呈现**:
   - 返回匹配的 message 列表？还是 session 列表？
   - 是否需要高亮匹配内容？
   - 是否需要分页？

### 6.3 技术决策

8. **Session 存储改进**:
   - 是否需要先修复 Pro 分支的原子写入问题？
   - 是否需要从 JSONL 迁移到 SQLite？

9. **主分支复用**:
   - main 分支的 `write_session_atomically` 是否可以 cherry-pick 到 pro？
   - 两个分支的 session 格式是否兼容？

---

## 7. 建议的 BMad 信息集转化

### PRD 结构建议

```
1. 功能概述
   - 日报/周报/月报自动生成与查看
   - 历史对话内容检索

2. 用户故事
   - [Story-1] 作为用户，我希望能查看 Diva 自动生成的日报/周报/月报
   - [Story-2] 作为用户，我希望能将报告固化为 SOP/Skill/Memory
   - [Story-3] 作为用户，我能让 Diva 搜索并回顾历史对话

3. 技术方案
   - Report 存储: [待确认]
   - Report 生成: [待确认]
   - Session 搜索: [待确认]

4. 依赖项
   - Session 持久化修复（原子写入）
   - [待确认] Memory provider 接口
   - [待确认] Skill 创建接口

5. 验收标准
   - [待确认]
```

---

## 8. 附件

- `NotebookView.vue` 完整代码: `agent-diva-gui/src/components/NotebookView.vue`
- Session Store: `agent-diva-core/src/session/store.rs`
- Session Manager (Pro): `agent-diva-core/src/session/manager.rs`
- Session Manager (Main): `agent-diva/agent-diva-core/src/session/manager.rs`
- Session History Research: `docs/logs/2026-06-session-research/session-history-storage-research.md`
