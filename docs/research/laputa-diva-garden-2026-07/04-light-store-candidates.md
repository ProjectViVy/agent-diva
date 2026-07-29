# 04 — Embedded Laputa 轻量本地存储候选比较

> **RG-RSCH-08b** · 对应研究问题 **Q8**
> 权威决策：[`laputa-memory-final-architecture.md`](../../architecture/laputa-memory-final-architecture.md)
> 日期：2026-07-24

## 1. 当前事实（Diva clean-break 基线）

| 事实 | 源码位置 |
|------|----------|
| Laputa 当前用 **单 key JSON blob** 存 proposal + section registry | `agent-diva-laputa/src/service.rs` — `NAMESPACE = "laputa.registry.v1"`, `REGISTRY_KEY = "memory"` |
| 持久化走 kernel `StateStore` CAS | `service.rs:361-376` — `compare_and_swap` |
| 有界：`MAX_SECTIONS=64`, `MAX_CONTENT_BYTES=64KiB`, `MAX_PROPOSALS=256` | `service.rs:15-19` |
| Profile 路径：`profiles/<id>/laputa.sqlite3` | `agent-diva-local/src/lib.rs:104`, `laputa_host.rs:172-174` |
| 底层为 `SqliteStateStore`（generic KV CAS） | `agent-diva-state/src/state_store.rs` |
| Workspace MSRV **Rust 1.80**，`rusqlite 0.31` + **`bundled`** | 根 `Cargo.toml:41,56` |
| Profile 下已有 **多个 SQLite 文件**（session/context/execution/laputa/…），共享同一 `rusqlite` 运行时 | `agent-diva-local/src/lib.rs:84-108` |
| **无 FTS5**、无全文索引、无 tombstone/outbox 表结构 | 全仓 `rg FTS5` 无命中 |
| `LaputaMemoryProvider` 只读投影，无检索 API | `agent-diva-laputa/src/provider.rs` |

**缺口**：当前 store 仅支持 section 级完整替换 revision，不支持 record 级检索、同步 journal、压缩层、或 Core Personality 物化视图。

## 2. 约束（来自 ADR + handoff）

- Rust **1.80**，Windows / Linux / macOS
- Profile-local，单进程，默认离线
- **不引入第二个 SQLite 运行时**（即不叠加 `sqlx`+`libsqlite3` 与 `rusqlite bundled` 双链）
- **不提升 MSRV**（排除 `memtle` 等 1.88+ 依赖）
- 在线权威 **不是** Markdown 目录双写
- Effectful 写入仍经唯一 `ToolExecutionGateway`（`agent-diva-execution`）
- ADR **未预定** SQLite/FTS5 胜出 — 本报告给出证据化比较

## 3. 候选方案

### 3.1 候选 A — SQLite + FTS5（单库多表 + 虚拟 FTS 表）

**形态**

```text
profiles/<id>/laputa.sqlite3
├── memory_records      (typed rows: id, section, kind, revision, tombstone, …)
├── memory_fts          (FTS5 VIRTUAL TABLE, content=memory_records)
├── sync_outbox / sync_inbox
├── persona_capsule     (materialized core subset)
└── schema_meta
```

**实现要点**

- 复用现有 `rusqlite` + `bundled`；FTS5 在 bundled SQLite 3.x 中默认可用（需 CI 验证 Windows）
- 检索：`MATCH` + `bm25()` 排序；metadata filter（section/kind/scope）
- Revision：行级 `revision` + `content_hash`；CAS 可用 `UPDATE … WHERE revision=?` 事务
- 新 runtime 只读取 typed record schema；当前 registry/blob 若需保留历史价值，仅能由离线 `agent-diva-migrate` 显式转换

**优点**

- 单文件、成熟、与 Diva 现有 profile SQLite 生态一致
- FTS5 对中英文 keyword/BM25 有 baseline 质量，**不依赖 embedding**
- 事务 + WAL 支持 outbox/inbox 与 record 同事务提交

**缺点 / 风险**

- FTS5 中文分词质量依赖 `tokenize=unicode61` 或自定义 tokenizer；无 embedding 时长 query 召回有限
- 同 profile 多 `.sqlite3` 文件已存在；若坚持「单文件」需合并或接受「单运行时、多文件」
- Schema 迁移需显式 version gate（`agent-diva-state` 已有 `SCHEMA_VERSION` 模式）

### 3.2 候选 B — SQLite 普通 B-tree 索引 + 应用层评分（无 FTS5）

**形态**

```text
memory_records(content TEXT, …)
memory_tokens(record_id, token, pos)   -- 或 trigram 表
memory_tags / section / kind indexes
```

**评分（应用层）**

```text
score = w_section * section_match
      + w_kind * kind_match
      + w_recency * exp(-age/τ)
      + w_importance * importance
      + w_exact * token_overlap(query, content)
```

**优点**

- 无 FTS5 扩展依赖风险（部分打包环境 FTS 被剥离 — 虽 bundled 通常可用）
- 评分公式完全可控，便于与 **ReplicationPolicy**、persona 权重对齐
- 更易单元测试（确定性 fixture）

**缺点**

- 需自建 tokenization / 中文 n-gram；开发量比 FTS5 大
- 大规模 corpus 下性能需 benchmark；不如 FTS5 内置倒排成熟
- 仍占用 SQLite 连接 — 与候选 A 运维特征相同

### 3.3 候选 C — Append-only event log + 派生检索索引

**形态**

```text
profiles/<id>/laputa/
├── events.log.jsonl     (append-only: Proposed/Decided/Migrated/Retracted/SyncApplied)
├── snapshots/           (periodic compacted state, optional)
└── index.sqlite3        (derived, rebuildable from log)
```

**优点**

- **审计友好**：与 governance spine（Receipt/Audit）同向
- 索引损坏可 **全量重建**；sync cursor 自然单调
- 主权威可以是 log + CAS snapshot，符合「evidence lineage」

**缺点**

- 在线检索仍需要 **某种** 索引（通常仍是 SQLite）→ 实际为「log + SQLite derived」，复杂度最高
- Compaction 作业需后台或启动时增量；Windows 文件锁与 log rotation 要设计
- 与当前 `StateStore` 单 blob CAS 差异大，迁移成本高

### 3.4 候选 D（补充）— 纯 KV StateStore 扩展（status quo++）

**形态**：延续 `laputa.registry.v1` JSON，增加第二个 namespace `laputa.records.v1` 分 key 存储。

**优点**：最小 diff，与 R7 Laputa 一致。

**缺点**：无原生全文检索；scan 256 key 上限内可接受，但 **无法** 支撑 MemoryOS 级检索与 sync journal；**不满足** E8-S4 目标。

## 4. 比较矩阵

| 维度 | A: SQLite+FTS5 | B: SQLite+app score | C: append-only+derived | D: KV++ |
|------|----------------|---------------------|------------------------|---------|
| Rust 1.80 / bundled | ✅ | ✅ | ✅（derived 仍 SQLite） | ✅ |
| Windows CI 风险 | 低（需实测 FTS5） | 最低 | 中（log IO） | 最低 |
| 第二 SQLite **运行时** | 否 | 否 | 否（同 rusqlite） | 否 |
| 单 profile 文件数 | 1 库（或合并） | 1 库 | 1+ 文件 | 1 库 |
| 无 embedding 检索 | BM25+filter 可用 | 自定义 overlap 可用 | 取决于 derived | ❌ 仅 key scan |
| Revision/CAS/tombstone | 表级事务 | 同左 | log 天然 + 表投影 | blob CAS 已有 |
| Sync outbox 同事务 | ✅ | ✅ | ✅ 最强 | 需新设计 |
| 与现有 LaputaService | 渐进迁移 | 渐进迁移 | 大改 | 最小 |
| 实现工作量 | 中 | 中高 | 高 | 低（不足） |
| 审计/重建 | 中 | 中 | **高** | 低 |

## 5. 取舍与风险

| 风险 | 缓解 |
|------|------|
| FTS5 中文召回差 | Phase 1 用 unicode61 + bigram fallback；Q9 用 hybrid rank |
| 多 SQLite 文件 | ADR 要求 profile-local，**非**强制单文件；文档统一称「单 rusqlite 运行时」 |
| 与 Garden 双权威 | 本地 store 只持有 **LocalCore/Cached** 策略记录；见 `07`、`08` |
| Gateway 绕过 | 所有 mutate 经 Laputa command → Gateway tool adapter，不暴露 raw SQL 给 UI |
| Schema 漂移 | `schema_version` + fail-closed open（对齐 `agent-diva-state`） |

## 6. 推荐结论（**非最终裁定**，供 synthesis 裁决）

**推荐路径：候选 A 为主，候选 B 的评分层为叠加（A+B hybrid）**

理由：

1. Diva 已在 profile 全面使用 `SqliteStateStore`；FTS5 在 bundled 下 **不增加** 第二运行时。
2. 无 embedding 时 FTS5 `bm25` + metadata filter 是可验收 baseline（回答 Q9）。
3. 应用层 scoring（B）可作为 **rerank** 层注入 importance/recency/section policy，而不替代倒排。
4. 候选 C 的 append-only **语义** 应用于 **sync_outbox 与 audit trail**，不必作为主存储范式（降低复杂度）。
5. 候选 D 仅适合 R7 最小 Laputa；**不足以**完成 E8-S4。

**分阶段**

- **S4b-1**：在 `laputa.sqlite3` 增加 `memory_records` + FTS5；不保留 registry/blob 兼容读
- **S4b-2**：`persona_capsule` 物化表 + sync outbox/inbox 表
- **S4b-3**：可选 embedding 侧car 表（Q10），FTS 仍为 default

## 7. 被否决方案

| 方案 | 否决原因 |
|------|----------|
| 引入 `memtle` / 第二 Rust SQLite 栈 | MSRV 1.88+；ADR Mentle clean-break |
| Markdown 目录为在线权威 | ADR 明确禁止 |
| 独立进程索引服务（Tantivy server 等） | 违背 embedded/offline-first |
| 纯内存 +  periodic flush | 崩溃丢 sync outbox；违反 ADR「写入与 outbound event 不可分离」 |
| 仅 KV scan（候选 D） | 无检索、无 sync journal，不满足 MemoryOS embedded 目标 |

## 8. 验收基准（供 synthesis / CI 引用）

| 用例 | 通过标准 | 命令/位置 |
|------|----------|-----------|
| FTS5 可用性 | `CREATE VIRTUAL TABLE t USING fts5(c); INSERT… MATCH` 三平台 green | `agent-diva-state` Laputa adapter 集成测试 |
| CAS 并发 | 两 task 同时 migrate 同 section，一者 `Conflict` | 复用 `state_store.rs` CAS 测试模式 |
| Outbox crash | kill -9 后重启，pending outbox ≥1 且 record 已提交 | laputa integration test |
| 单 profile 体量 | 10k records 全 scan + top-8 search P95 < 200ms（debug 可放宽） | criterion 可选 |
| MSRV | `cargo +1.80.0 check -p agent-diva-state` | workspace gate |
| 第二运行时 | `cargo tree -i rusqlite` 仅一条链 | deletion-proof |

**Windows 特别项**：Tauri 打包后 bundled SQLite 与 dev 一致；FTS5 在 `agent-diva-gui/src-tauri` CI matrix 必跑一项 smoke。

## 9. 开放假设

- [ ] Windows release 构建下 `CREATE VIRTUAL TABLE … USING fts5` 实测通过
- [ ] `laputa.sqlite3` 与现有 registry blob **共存迁移** 的一次性 upgrade 脚本行为
- [ ] 单 profile Laputa 记录量级上限（建议初始 **≤10k records / ≤32MiB**）需 stakeholder 确认
- [ ] 是否与 session/context DB **物理合并** 为单文件 — 运维 vs 隔离 tradeoff 未决
- [ ] Garden 下行 replication 是否写入同一 DB 或独立 `sync_inbox` 命名空间

## 10. 研究问题 Q8 回答

**Q8：本地 store 候选有哪些？为何选择最终方案？**

- **候选**：A SQLite+FTS5、B SQLite+应用层评分、C append-only+派生索引、（对照）D KV++。
- **推荐**：**A+B hybrid** — FTS5 负责倒排与 BM25 baseline，应用层负责 policy-weight rerank；sync/audit 借鉴 C 的 append-only **语义**但不采用 C 为主存储。
- **未预定 SQLite 必然胜出**：若 Windows FTS5 CI 失败，fallback 为 **候选 B**（放弃 FTS5，全应用层索引），仍保持 SQLite 单运行时；仅当 SQLite 整体不可接受时才回退到 C（概率低）。
