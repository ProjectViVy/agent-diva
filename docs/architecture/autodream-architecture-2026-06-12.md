---
stepsCompleted: [1]
lastStep: 1
status: 'in-progress'
revision: v1.1 (2026-06-12, scope-merge with Report System)
inputDocuments:
  - docs/prds/prd-autodream-2026-06-12/prd.md
  - docs/prds/prd-autodream-2026-06-12/.decision-log.md
  - docs/dev/genericagent/autodream-rhythm-distillation-design.md
  - docs/dev/genericagent/compression-research.md
  - docs/dev/genericagent/autonomous-evolution-simplified-architecture-decision.md
  - docs/dev/genericagent/compression-taxonomy-decision.md
  - docs/dev/genericagent/context-compaction-vs-autonomous-evolution-decision.md
  - docs/prd-report-system/prd.md (新增 v1.1 引用)
  - docs/architecture/scope-merge-decision.md (新增)
workflowType: 'architecture'
hard_constraints:
  - "existing workspace — stack locked: Rust 2021, Vue 3 + Tauri v2, 14 crates"
  - "v1.1 新增: AutoDream 与 Report System 解耦契约 (ADR-008)"
  - "token budget: unlimited"
---
project_name: 'agent-diva-pro'
user_name: '大湿'
date: '2026-06-12'
---

# Architecture Decision Document — AutoDream

_本文档记录 AutoDream 节律性反思蒸馏机制的架构决策。技术栈锁定，聚焦增量决策。_

---

## Project Context Analysis

### Requirements Overview

**Functional Requirements (14 FRs):**

| 模块 | FRs | 内容 | 架构影响 |
|------|-----|------|----------|
| 触发与调度 | FR-1~2 | 手动触发 + 时间门自动触发 | `agent-diva-autodream::trigger` |
| 并发控制 | FR-3~4 | Lock + checkpoint 机制 | `agent-diva-autodream::concurrency` |
| 输入收集 | FR-5 | 多源输入优先级读取 | `agent-diva-autodream::inputs` |
| 蒸馏执行 | FR-6~7 | Forked subagent + 四阶段 prompt | `agent-diva-autodream::distiller` |
| 产物输出 | FR-8~9 | 结构化 JSON + 事件流 | `agent-diva-autodream::outputs` |
| 审查与确认 | FR-10~11 | 候选缓存 + 用户决策 | `agent-diva-autodream::review` + GUI |
| 节律报告 | FR-12~13 | 日报 + 周报 | `agent-diva-autodream::reports` |
| 失败处理 | FR-14 | 失败通知 + 重试 | `agent-diva-autodream::error` |

**Non-Functional Requirements:**
- 不阻塞主循环（异步执行）
- 可审计（产物结构化 + 事件流）
- 可重跑（checkpoint + lock）
- 向后兼容（不修改现有 consolidation）

---

## ADR-001: 模块归属 — 新建 `agent-diva-autodream` crate

### 决策

新建独立 crate `agent-diva-autodream`，不并入现有 crate。

### 理由

| 选项 | 优点 | 缺点 |
|------|------|------|
| 新建 crate | 职责清晰、可独立版本、编译隔离 | 增加 workspace 复杂度 |
| 并入 `agent-diva-agent` | 减少 crate 数量 | 职责模糊、编译耦合 |
| 并入 `agent-diva-core` | 共享核心类型 | 核心膨胀、循环依赖风险 |

### 决策理由

AutoDream 是独立的后台反思机制，与 agent loop、memory、GUI 都有交互但无强耦合。独立 crate 符合单一职责原则，便于未来扩展（如支持多租户、mask 隔离等）。

### 影响

- Workspace 增加 1 个 crate
- Cargo.toml 新增 `[workspace.members]` 条目
- 其他 crate 通过 API 调用，不直接依赖

---

## ADR-002: API 契约 — Tauri Command 定义

### 决策

定义 6 个 Tauri command，由 `agent-diva-autodream` crate 提供。

| Command | 输入 | 输出 | 用途 |
|-----------|------|------|------|
| `trigger_autodream` | `{ trigger_type: 'manual' \| 'scheduled' }` | `{ task_id: string, status: 'running' \| 'queued' }` | 手动触发 |
| `get_autodream_status` | `{ task_id: string }` | `{ status, progress?, error? }` | 轮询状态 |
| `get_autodream_candidates` | `{ status?, limit? }` | `MemoryCandidate[]` | 获取候选 |
| `review_candidate` | `{ candidate_id, action, modified_content? }` | `{ success }` | 审查候选 |
| `get_autodream_reports` | `{ period, date? }` | `NotebookReport[]` | 获取报告 |
| `cancel_autodream` | `{ task_id }` | `{ success }` | 取消运行 |

### 理由

复用现有 Tauri command 模式（如 `SelfEvolutionSettings.vue` 中的 `get_self_evolution_config`），保持前后端一致性。

### 影响

- `agent-diva-gui/src-tauri/src/commands.rs` 新增 command handler
- `agent-diva-autodream/src/lib.rs` 暴露 public API

---

## ADR-003: 执行模型 — Forked Subagent

### 决策

AutoDream 在独立线程中运行，通过线程间通信与主循环交互。

### 理由

| 选项 | 优点 | 缺点 |
|------|------|------|
| 独立线程 | 实现简单、不阻塞主循环 | 资源占用、需手动管理生命周期 |
| 异步任务 (tokio::spawn) | 与现有 async runtime 集成 | 可能阻塞 tokio worker |
| 独立进程 | 完全隔离、可 kill | 通信复杂、启动开销大 |

### 决策理由

独立线程是最平衡的选择。AutoDream 是 CPU/IO 密集型任务，独立线程不会阻塞 tokio 的 async runtime，同时避免了进程间通信的复杂性。

### 影响

- `agent-diva-autodream/src/worker.rs` 实现 Worker 线程
- 通过 `mpsc` 通道与主线程通信
- 支持 cancel token 取消运行

---

## ADR-004: 输入源读取策略

### 决策

按优先级顺序读取，总 token 数超过预算时按优先级截断。

| 优先级 | 输入源 | 读取边界 |
|--------|--------|----------|
| 1 | 近期会话（session store） | 最近 N 个会话 |
| 2 | HISTORY.md | 最近 M 条记录 |
| 3 | MEMORY.md | 完整读取 |
| 4 | Source Capsules | 如有 |

### 理由

近期会话包含最新上下文，优先级最高。MEMORY.md 是长期记忆，完整读取以确保蒸馏的连贯性。

### 影响

- `agent-diva-autodream/src/inputs.rs` 实现输入源读取
- 每个输入源有独立的读取边界配置
- 总 token 数超过预算时，低优先级输入源被截断

---

## ADR-005: 产物输出格式

### 决策

产物为 JSON 格式，包含结构化字段。

```json
{
  "memory_patch_candidates": [
    {
      "id": "uuid",
      "content": "string",
      "confidence": 0.95,
      "evidence_refs": ["session_id:msg_idx"],
      "review_required": true
    }
  ],
  "journal_entries": [...],
  "learning_candidates": [...],
  "evidence_refs": [...],
  "confidence": 0.92
}
```

### 理由

JSON 格式便于前后端解析，结构化字段支持细粒度的审查和审计。

### 影响

- `agent-diva-autodream/src/outputs.rs` 实现产物生成
- 产物 schema 版本化（`v1`）
- 事件流追加到 `events.jsonl`

---

## ADR-006: 失败处理策略

### 决策

失败时自动重试 3 次，每次退避 5 分钟。重试失败时通知用户。

### 理由

| 选项 | 优点 | 缺点 |
|------|------|------|
| 立即重试 | 快速恢复 | 可能连续失败 |
| 退避重试 | 避免雪崩 | 延迟较长 |
| 不重试 | 简单 | 用户需手动触发 |

### 决策理由

退避重试是最平衡的策略。3 次重试 × 5 分钟退避 = 最大 15 分钟延迟，可接受。

### 影响

- `agent-diva-autodream/src/error.rs` 实现重试逻辑
- 失败记录到 `events.jsonl`
- GUI 通过 `showAppToast` 通知用户

---

## ADR-007: 与现有系统的集成点

### 决策 (v1.1 修订)

| 系统 | 集成方式 | 说明 |
|------|----------|------|
| agent loop | 事件触发 | AutoDream 运行完成后发送事件 |
| consolidation | 无直接集成 | 独立运行，不替代 |
| MEMORY.md | 审查后写入 | 候选 → 用户确认 → 写入 |
| NotebookView | **不直接修改** | AutoDream 仅写入 markdown 文件，NotebookView 由 Report System PRD 负责 |
| SelfEvolutionSettings | Tauri command | 配置频率和阈值 |
| **Report System** (v1.1 新增) | **写入路径契约** | AutoDream 写入 `.agent-diva/autodream/reports/*`，由 Report System 消费展示 |

### 理由 (v1.1 修订)

保持边界清晰。AutoDream 是独立的后台机制（**数据压缩层**），与现有系统通过事件和 API 交互。AutoDream **不直接修改** Report System 的 NotebookView 代码——通过文件路径契约解耦，符合"autodream 只是压缩技术，最终回归用户可见内容"的边界原则。

### 影响

- `agent-diva-autodream/src/events.rs` 实现事件发布
- 其他系统通过事件订阅或 API 调用获取 AutoDream 状态
- **v1.1 新增**: `agent-diva-autodream/src/outputs.rs` 暴露 `write_rhythm_report(period, content)` 公共 API

---

## ADR-008: 与 Report System PRD 的协作契约 (v1.1 新增)

### 决策

AutoDream 产出的日/周报 markdown 文件落地到固定路径后即视为交付完成，**不直接修改** NotebookView。
Report System 负责统一展示、固化、搜索。

> 用户原话 (2026-06-12): "autodream 只是一个压缩技术，而最终还是要回归到用户的可见内容中"

### 契约

**写入路径**:
- 日报: `.agent-diva/autodream/reports/daily/{YYYY-MM-DD}.md`
- 周报: `.agent-diva/autodream/reports/weekly/{YYYY-Www}.md`

**写入时机**: 蒸馏运行完成、report 章节产出后立即写入。

**写入方式**: 原子写入（write to `.tmp` → `rename`），与 Session 原子写入修复保持一致。

**失败处理**: AutoDream 侧重试 3 次（与现有 ADR-006 退避重试策略一致），仍失败则记录到 `events.jsonl` 并通过 `showAppToast` 通知用户。**不**触发 Report System 任何回退。

**读侧契约**: NotebookView 周期性扫描该路径（每次切换 daily/weekly 标签时触发），**不订阅** AutoDream 事件。扫描失败时降级到"暂无报告"提示。

**Frontmatter Schema** (v1):
```yaml
---
period: daily | weekly
date: YYYY-MM-DD
week: YYYY-Www (仅 weekly)
generated_at: ISO8601 timestamp
generated_by: agent-diva-autodream
schema_version: 1
---
```

### 影响

- AutoDream 侧: `outputs.rs` 增加 `write_rhythm_report(period, content)` API，签名见下
- Report System 侧: 文档化 read 路径，**不**依赖 AutoDream 内部模块
- 解耦: 两侧可独立部署、独立测试

### API 签名

```rust
// agent-diva-autodream/src/outputs.rs
pub async fn write_rhythm_report(
    period: RhythmPeriod,  // Daily | Weekly
    date_or_week: String,  // "2026-06-12" 或 "2026-W24"
    content: String,
) -> Result<PathBuf, WriteError>;

pub enum RhythmPeriod {
    Daily,
    Weekly,
}
```

### 替代方案对比

| 方案 | 优点 | 缺点 |
|------|------|------|
| **选定**: 文件路径契约 | 解耦、独立部署、零运行时依赖 | 跨进程一致性较弱（靠文件系统） |
| 共享 crate (auto-link) | 类型安全、IDE 提示强 | 增加编译耦合、违反 PRD 边界 |
| Tauri command 互调 | 复用现有 command 模式 | 引入跨 crate API 表面，仍是耦合 |
| 事件总线 + 订阅 | 解耦 + 实时性 | 增加事件流复杂度、GUI 需订阅 |

选定文件路径契约的核心理由：**AutoDream 是异步后台任务，Report System 是用户交互层，两者的运行时机不一致，文件路径契约最符合"产出-消费"的时间解耦语义**。

---

## Next Steps

1. **创建 `agent-diva-autodream` crate** — `cargo new --lib agent-diva-autodream`
2. **实现核心模块** — trigger, concurrency, inputs, distiller, outputs, review, reports, error
3. **注册 Tauri command** — `agent-diva-gui/src-tauri/src/commands.rs`
4. **扩展 GUI** — NotebookView 支持候选审查，SelfEvolutionSettings 支持触发配置
5. **集成测试** — 验证端到端流程
6. **v1.1 新增** — 实现 `write_rhythm_report` API (ADR-008) + Report System 路径契约测试
