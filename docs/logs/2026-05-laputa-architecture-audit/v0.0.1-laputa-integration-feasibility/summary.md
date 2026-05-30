# Laputa-work 架构调研：最小化接入 agent-diva 可行性

## 版本信息
- 版本号: v0.0.1-laputa-architecture-audit
- 调研日期: 2026-05-28
- 范围: `C:\Users\Administrator\Desktop\laputa-work\`（laputa-next、laputa、mempalace-origin）
- 约束: 只读调研，不修改业务代码

---

## 核心结论

**Laputa-next 已经是最小化设计（Laputa Lite），架构方向正确，可以直接作为 agent-diva 的人格连续性薄层接入。节律设计不复杂（日历驱动），UPSP 的复杂部分已被明确拒绝。**

---

## 研究背景

上一轮调研（v0.0.1-architecture-analysis）确认 Phase 1 应为纯文件 + 公理，mentle 降为 Phase 2 存储引擎。本轮转向 laputa-work，评估：
1. Laputa 能否替代 agent-diva 的多文件结构（MEMORY.md / HISTORY.md / SOUL.md / BOOTSTRAP.md）
2. 节律设计是否过复杂
3. UPSP 概念中哪些值得保留、哪些应拒绝
4. 已开发内容的状态

---

## 关键发现

### 发现 1：2026-05-13 已完成架构重置

Laputa 从"全量集成 Mempalace"改为"Laputa Lite"方案（`docs/IMPORTANT-laputa-lite-mempalace-toolkit.md`）：

- Laputa = 文件态人格 + 节律 + 唤醒投影（薄层，不持有 SQL）
- Mempalace = 存储引擎（独立 toolkit，Laputa 可选调用）
- Agent-Diva 调用 Laputa 工具做人格，调用 Mempalace 工具做记忆

**DECITION.md D-001 明确**："Laputa is the upper personality database and scheduling layer for loop agents. It is not a replacement for Mempalace."

### 发现 2：代码全部完成，看板停在 Phase 0

- 85 个 Rust 文件，零 TODO/stub，全部生产就绪
- 看板（IMPORTANT-laputa-lite-kanban.md）Phase 1-5 全部未开始
- 阻塞在 Phase 0 的 vendor/mempalace 清理

### 发现 3：节律设计并不复杂

**复杂的节律来自 UPSP（已拒绝）：**
- 32 轮节律点、动态六轴 20 区间、疲劳值双阈值、工化指数四维几何平均

**Laputa-next 的节律是纯日历驱动（D-012）：**
- 每日：heat 衰减 + 当日证据摘要（模板，不需要 LLM）
- 每周：7 天证据 + 胶囊（可选 LLM） + heat +3 + delta 候选
- 每月：30 天证据 + 编年（可选 LLM） + 基线 +1 + delta 固化

触发方式：RhythmService 比较 `index.toml` 中的 last daily/weekly/monthly 与当前日期，无需轮数计算。

### 发现 4：日报/周报/月报 = Laputa Rhythm 的精确实现

| 报告类型 | Laputa 实现 | 进化触发 |
|---|---|---|
| 日报 | `run_daily` → `.laputa/rhythm/daily/YYYY-MM-DD.md` | 证据积累（不直接进化） |
| 周报 | `run_weekly` → `.laputa/rhythm/weekly/YYYY-WNN.md` | 热房间 +3、delta 候选、未解决线程清理 |
| 月报 | `run_monthly` → `.laputa/rhythm/monthly/YYYY-MM.md` | 基线 +1、delta 固化、进化历史、房间晋升/冷却 |

**D-018**："Weekly and monthly outputs are not cosmetic reports. They are the primary triggers that update Laputa personality state."

### 发现 5：UPSP 概念评估

**✅ 保留（Laputa 已吸收）：**
- 主体性延续（WakeupPack + 身份连续性）
- 节律点机制（RhythmEngine）
- 关系共振（Relationship Weather）
- 人格状态 > 静态文本（SOUL.md 作为 projection）

**❌ 明确拒绝（D-009）：**
- 七文件结构（与 Mempalace 重复存储）
- 核心六轴 SCVARB（过度建模）
- 动态六轴每轮更新（高维护低收益）
- 疲劳值 + 睡眠周期（不适用 agent-diva）
- 32 轮节律点（日历驱动更简单）
- STM/LTM 分离存储（Mempalace drawer 已统一）

**⚠️ UPSP 中"扯淡"的部分：**
1. **工化指数**：四维复合几何平均，无消费者，纯自嗨
2. **动态六轴**：arousal=0.6 无人关心
3. **32 轮节律点**：硬编码与实际对话长度无关
4. **跨智能体适配器**：Zeroclaw/Openfang 适配都是空气代码
5. **"主体性工程"包装**：概念包装 > 工程价值

### 发现 6：Laputa 可替代 agent-diva 多文件结构

| agent-diva 文件 | Laputa 替代 |
|---|---|
| `MEMORY.md` | WakeupPack（动态生成：热房间 + 最新胶囊 + 记忆提示） |
| `HISTORY.md` | Mempalace drawer（diary room，每日证据自动归档） |
| `SOUL.md`（手写） | SOUL.md projection（7 节，Laputa 自动生成） |
| `BOOTSTRAP.md` | identity.md（人写，Laputa 管理） |

**不能替代**：`memory/*.md` SOP（GenericAgent L3），属于"知识"不属于"人格"。

---

## 架构判断

### Laputa 在 agent-diva 中的定位

```
┌─────────────────────────────────────────────────────────┐
│                    Agent 层（agent-diva-agent）           │
├─────────────────────────────────────────────────────────┤
│          MemoryProvider trait（4 个钩子）                 │
│  system_prompt_block │ prefetch │ sync_turn │ on_end    │
├──────────────────────┬──────────────────────────────────┤
│   Laputa（人格薄层）  │   通用记忆层（GenericAgent 风格） │
│   - wakeup           │   - L0 公理                      │
│   - soul projection  │   - L1 索引（≤30行）             │
│   - rhythm (D/W/M)   │   - L2 事实                      │
│   - heat model       │   - L3 SOP                       │
│   - identity delta   │                                  │
├──────────────────────┴──────────────────────────────────┤
│          Mempalace（可选存储引擎，Phase 2）               │
│          SQLite + BM25 + KG + drawers                   │
└─────────────────────────────────────────────────────────┘
```

**关键判断**：Laputa（人格连续性）和 GenericAgent 风格记忆（知识管理）是**正交的两层**，可以独立演进。

---

## 推荐接入路径

### Phase 0：清理 vendor/mempalace
- 导出当前 diff
- 恢复脏文件
- 确认 `cargo check --workspace`

### Phase 1：Laputa 文件态闭环
- MemorySink/MemoryContext 端口（NoopMemorySink）
- sync_turn 文件态优先
- rhythm 写 `.laputa/rhythm/*`
- wakeup/soul 只依赖 `.laputa`

### Phase 2：agent-diva 接入
- LaputaMemoryProvider 实现 MemoryProvider trait
- system_prompt_block → wakeup
- sync_turn → sync_turn
- on_session_end → rhythm

### Phase 3：通用记忆层保留
- memory/*.md SOP 独立于 Laputa
- GenericAgent 公理 + 分类决策树
- 与 Laputa 人格层正交共存

---

## 风险与治理

| 风险 | 影响 | 控制 |
|---|---|---|
| Laputa 人格层与通用记忆层职责模糊 | Laputa 又变成 memory proxy | D-001 边界清晰：人格归 Laputa，知识归记忆层 |
| rhythm 依赖 LLM 但 LLM 不可用 | 周报/月报无法生成 | TemplateCapsuleWriter 作为降级方案（已实现） |
| vendor/mempalace 脏文件恢复引入回归 | 构建失败 | Phase 0 先做，确认 check 通过再推进 |
| UPSP 概念悄悄回流 | 复杂度膨胀 | D-009 作为红线，新概念必须对照拒绝清单 |

---

## 参考文献

| 文档 | 路径 |
|---|---|
| Laputa Lite 架构设计 | `laputa-next/docs/IMPORTANT-laputa-lite-mempalace-toolkit.md` |
| Laputa Lite 项目管理 | `laputa-next/docs/IMPORTANT-laputa-lite-project-management.md` |
| Laputa Lite 看板 | `laputa-next/docs/IMPORTANT-laputa-lite-kanban.md` |
| 架构决策记录（D-001~D-026） | `laputa-next/docs/dev/DECITION.md` |
| 架构概述 | `laputa-next/docs/dev/architecture.md` |
| Living Memory 契约 | `laputa-next/docs/dev/living-memory.md` |
| UPSP 兼容性分析 | `laputa-next/docs/dev/legacy/upsp-study/UPSP与DIVA-Soul兼容性分析.md` |
| UPSP 执行摘要 | `laputa-next/docs/dev/legacy/upsp-study/executive-summary.md` |
| 上一轮调研 | `docs/logs/2026-05-memory-architecture-deep-dive/v0.0.1-architecture-analysis/summary.md` |

本版本为调研沉淀文档，不含代码变更。
