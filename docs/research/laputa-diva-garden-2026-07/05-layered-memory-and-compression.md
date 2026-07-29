# 05 — 分层记忆、压缩与所有权边界

> **RG-RSCH-08b/c** · 对应研究问题 **Q11–Q13**
> 交叉引用：`agent-diva-session`、`agent-diva-context`、`agent-diva-laputa`
> 日期：2026-07-24

## 1. 当前事实：谁拥有什么

### 1.1 SessionStore（L4 工作证据）

| 职责 | 证据 |
|------|------|
| Turn 生命周期、transcript、Session 层级（Root/Branch/Subagent/Ephemeral/**Compression**） | `agent-diva-session/src/lib.rs:26-78` |
| Schema v3 SQLite：`sessions.sqlite3` | `agent-diva-local/src/lib.rs:84` |
| Context 组装读 transcript + provider blocks | `agent-diva-session` context 模块 |

**结论**：**原始对话证据、turn 级 working history 归 SessionStore**，不归 Laputa。

### 1.2 Context Retention（working memory / 会话内压缩）

| 职责 | 证据 |
|------|------|
| Compact：模型生成 summary，标记 `excluded_history_ids` | `agent-diva-context/src/lib.rs:300-312` |
| Retain：用户/策略显式保留块 | `lib.rs:368-384` |
| Clear：排除 history 但不删 transcript | `excluded_history_ids` |
| 命名空间 `context.retention.v1`，per-session key | `lib.rs:16`, `400-416` |
| `RetentionContextProvider` → `ContextCategory::Summary` / `Memory`（**非 Laputa**） | `lib.rs:329-386` |

**结论**：**会话内 working checkpoint、compaction summary、显式 retain 块归 Context**，是 **transient-to-session** 的 working memory，不是 profile-global 长期记忆。

### 1.3 Laputa（长期记忆 governance + 投影）

| 职责 | 证据 |
|------|------|
| Proposal → Decision → Migration → **immutable section revision** | `agent-diva-laputa/src/service.rs` |
| Profile / Session scope | `types.rs:7-15` |
| `LaputaMemoryProvider` 只读 committed sections | `provider.rs:22-52` |
| **无** compression、无 layered index、无 FTS | 当前 crate |

**结论**：Laputa 拥有 **经 governance commits 的 long-term memory sections**；尚未实现 L0–L4 物理分层。

## 2. 目标分层模型（Adapt from GenericAgent，Reject 文件直写）

```text
L0  稳定治理规则（kernel/governance 常量 + policy，非用户数据）
L1  Bootstrap / existence index（极小，常驻 Context）
L2  Core personality & durable facts（PersonaCapsule + LocalCore records）
L3  Experience / SOP / reusable knowledge（按需检索）
L4  Session evidence & archives（SessionStore 权威；Laputa 仅指针/摘要）
```

| 层 | 所有者 | 持久化 | Context 默认策略 |
|----|--------|--------|------------------|
| L0 | governance/runtime config | 代码 + profile config | Required policy blocks |
| L1 | **Laputa-Diva**（derived） | `laputa.sqlite3` index 表 | Required，硬 cap（见 Q11） |
| L2 | **Laputa-Diva** | memory_records + persona_capsule | Required（offline boot） |
| L3 | **Laputa-Diva**（local subset）+ Garden（remote full） | local cached + Garden authority | Optional，retrieve on demand |
| L4 | **SessionStore** | `sessions.sqlite3` | Transcript via session assembler |
| Working | **Context retention** | `context.sqlite3` per session | Summary/Retain blocks |

**Reject**：GenericAgent 模型直接写 Markdown；Diva 必须 proposal + Gateway + CAS。

### 2.1 L0 层（治理常量，非用户 corpus）

| 内容 | 所有者 | 示例 |
|------|--------|------|
| Authority spine 规则 | kernel + governance | 无 bypass Gateway |
| ReplicationPolicy 默认 | Laputa config namespace | `07` 矩阵 |
| Caps / schema version | `LaputaService` 常量 | `MAX_CONTENT_BYTES` 等 |
| Skill signed blocks | `agent-diva-skill` | 非 Laputa，但 Context Required |

L0 **不**进入 FTS 索引；仅配置与 policy 投影。

### 2.2 GenericAgent 对照（Keep / Adapt / Reject）

| GenericAgent 机制 | Diva 映射 | 判定 |
|-------------------|-----------|------|
| L1 global index 常驻 | `L1` projector + Required Context | **Adapt**（工程化投影） |
| L2/L3 按需读文件 | FTS search + Optional Context | **Adapt** |
| working checkpoint | Context retention Compact | **Adapt**（已有 crate） |
| long-term update 模型写文件 | Laputa propose → migrate | **Adapt** |
| 模型纪律维持索引 | L1 projector 确定性 | **Reject** 纯模型 |
| 压缩删原文件 | Session 不物理删；Laputa tombstone | **Reject** GA 做法 |

## 3. Q11 — L1/bootstrap 如何保持有界和确定性

### 3.1 L1 定义

**L1 = Memory Existence Index**：每条为 `(record_id, section, kind, title_line, importance, updated_at)`，**不含**全文。

### 3.2 有界规则（推荐）

| 参数 | 建议值 | 依据 |
|------|--------|------|
| L1 最大条目 | **128** | GenericAgent global index 极小；Diva 需覆盖 14 section 头指针 |
| 单条 L1 字节 | **≤256 B** | 保证整 index < 32KiB |
| 总 L1 token 预算 | **≤800 tokens** Context injection | 对齐 embedded 产品 |
| 更新触发 | record **Committed** / **Retracted** / sync **Applied** | 确定性事件驱动 |
| 驱逐 | LRU by `importance` then `updated_at` | 超 cap 时 drop 最低分 |

### 3.3 确定性

- L1 **仅由 store 投影器** 从 `memory_records` 派生，禁止模型直接改 L1 文件
- 投影算法 versioned：`l1_projector_v1` — 同输入 revision 必得同 index
- 启动时：若 L1 与 records 不一致 → **rebuild from records**（fail-closed log）

### 3.4 Context 注入

- 新 `LaputaBootstrapProvider`（或扩展 `LaputaMemoryProvider`）注入 L1 block，`ContextBlockPolicy::Required`
- 格式：`[L1 index rev=N]\n- {section}/{kind}: {title} (id=…)`

## 4. Q12 — 逐级压缩由谁触发，依据是什么

### 4.1 触发主体

| 层级 | 触发者 | 机制 |
|------|--------|------|
| L4 → L3 指针 | **AutoDream / Evolution proposal** 或 scheduled job | 从 Session 挖掘 salient → `Laputa propose` |
| L3 内部合并 | **Laputa compression job**（local）或 **Garden**（connected） | 相同 kind 多条 → 单条 summary record + lineage |
| L2 persona 变更 | **Manual decision** 或 Garden **replication** | 禁止自动 LWW |
| Session transcript | **Context Compact**（session-scoped） | 已有 `agent-diva-context` Compact API |

**禁止**：Context Compact 直接写 Laputa section（避免 session summary 污染 long-term）。

### 4.2 触发条件（推荐）

```text
compress_candidate when ALL:
  - evidence_count >= 2 OR age > 30d
  - importance < promote_threshold
  - token_size > section_soft_cap
  - NOT persona/core kind
  - has provenance chain intact
```

| 信号 | 阈值（初始） |
|------|--------------|
| `importance` | 模型/规则打分 0–100；<40 可压 |
| token_size | section soft cap 8KiB；hard 64KiB（对齐 `MAX_CONTENT_BYTES`） |
| age | 30d 无 retrieval hit |
| retrieval_hits | 90d 零 hit → archive tier |

### 4.3 压缩产物

- 新 record revision **supersedes** 旧 ids（tombstone 旧 record）
- `lineage`: `{ "compressed_from": ["id1","id2"], "evidence_digest": "…" }`
- **保留**最高价值 source excerpt（≤512B）在 `provenance.excerpt`

### 4.4 Garden 分工

- Connected 模式：**deep compression** 在 Garden 执行；Diva 只接收 **replication policy 允许** 的压缩结果
- Offline：Diva 可运行 **lite compression**（规则 + 可选小模型），产出仍走 proposal/decide

## 5. Q13 — 如何保存最重要的数据来源和引用

### 5.1 Provenance 记录（每条 memory record）

```rust
struct MemoryProvenance {
    source_instance_id: String,   // diva profile / garden tenant
    origin: Origin,               // LocalProposal | GardenReplication | ImportPack
    evidence_digest: String,      // SHA-256 bounded evidence
    session_id: Option<String>,   // if from session mining
    turn_ids: Vec<String>,        // bounded list
    receipt_id: Option<String>,   // Gateway receipt when effectful
    proposal_digest: Option<String>,
    parent_record_ids: Vec<String>,
    created_at_unix_s: u64,
}
```

### 5.2 与现有 Laputa 对齐

- 已有 `proposal_digest` + `evidence` on proposal（`types.rs:28-30`, `service.rs:497-502`）
- 扩展：**migration 不删除** prior revision — 改为 tombstone + lineage（当前 `ReplaceMemoryMigration` 为全量替换，**需升级**）

### 5.3 反摘要循环污染（anti-summary-pollution）

| 规则 | 说明 |
|------|------|
| **No summary-of-summary** | 压缩产物 `kind=compressed_summary` 不得再作为 compression 输入 |
| **Min evidence depth** | 压缩链深度 ≤3；更深必须保留 L4 pointer |
| **Session summary firewall** | `ContextCategory::Summary` 内容 **不可** 自动 propose 到 Laputa |
| **Garden query firewall** | resolve 结果进 Context only； replication 才持久化（ADR） |
| **Digest binding** | 每条 record content hash 绑定 provenance；变 content 必新 revision |
| **Retract poisons merge** | tombstone record 不可被压缩合并复活 |

### 5.4 Session → Laputa 晋升路径

```text
Session turn evidence
  → (optional) Context Retain [session-scoped]
  → Explicit proposal with turn_ids + evidence digest
  → User/Garden approve
  → L3/L2 record + L1 index update
```

### 4.5 压缩作业状态机（L3 合并）

```text
Idle → ScanCandidates (scheduled/trigger)
     → ProposeCompression (create proposal + evidence digests)
     → AwaitDecision (auto-deny if policy=persona)
     → Migrate (tombstone sources + new compressed record)
     → ProjectL1 → Idle
```

- **Scheduled**：weekly job via `agent-diva-scheduler`（optional slice），仅 `Cached`/`L3` kind
- **Manual**：CLI `laputa compress --section memory_md --yes`
- **Garden**：Diva 不跑 deep merge；只 apply replication 下行产物

## 6. 候选方案比较

| 方案 | 优点 | 缺点 |
|------|------|------|
| **A. Laputa-owned L1–L3 + Session L4**（推荐） | 边界清晰，对齐 ADR | 需新 projector |
| B. Context 兼管 L1 | 少 crate | 混淆 working vs long-term |
| C. Garden 兼管 L1–L4 | 功能强 | 离线不可用，违反 Diva 哲学 |

## 7. 推荐结论

1. **所有权**：L4/working → Session+Context；L0–L3 → Laputa-Diva；Garden 持有 L3/L4 完整语料 **remote authority**。
2. **L1**：128 条 cap、事件驱动投影、启动可 rebuild。
3. **压缩**：Session compact ≠ Laputa compress；Laputa 压缩走 proposal + lineage + tombstone。
4. **Provenance**：mandatory `MemoryProvenance`；summary 防火墙 + 压缩深度 cap。

## 8. 被否决方案

- 模型每轮自动写 Laputa（无 proposal）— 违反 governance spine
- 将 HISTORY section 全量 mirror Session transcript — token/authority 双爆炸
- GenericAgent 式删除 L4 原文件 — `compress_session.py` 风险；Diva L4 永不物理删 transcript

## 9. 开放假设

- [ ] AutoDream 是否为 L4→L3 挖掘的默认 producer
- [ ] Compression 是否允许调用 Model（离线小模型 vs 纯规则）
- [ ] `history_md` section 在 Laputa-Diva 是否 **RemoteOnly**（见 `07`）
