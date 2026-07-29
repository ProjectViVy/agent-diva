# RG-RSCH-08 Implementation Outline — RG-E8-S4 Story DAG

> 研究完成 ≠ Laputa-Diva 实现完成。下列 Story 待 `synthesis.md` 签收后按 DAG 排期。
> **对齐 TODOLIST**：`RG-E8-S4a` / `S4b` / `S4c` · **禁止**跳过 S4a 或 Gateway 收敛前 bulk 写存储。
> **默认不自动开工**；编码前须 Keep 本 outline 与 synthesis 开放决策（§22）。

---

## Story DAG 总览

```mermaid
flowchart TD
  S4a[S4a Mentle clean-break]
  S4b1[S4b-1 Store + FTS + schema]
  S4b2[S4b-2 PersonaCapsule + L1 + Gateway Tools]
  S4b3[S4b-3 Local search + merge + status]
  S4b4[S4b-4 Memory Pack import/export (Deferred)]
  S4c1[S4c-1 Garden resolve client]
  S4c0[S4c-0 laputa-sync/1 contract]
  S4c2[S4c-2 Selective replication client]
  S4c3[S4c-3 Outbox push + sync FSM]

  S4a --> S4b1
  S4b1 --> S4b2
  S4b2 --> S4b3
  S4b2 -. deferred / RG-E5 .-> S4b4
  S4b3 --> S4c1
  S4c0 --> S4c2
  S4c1 --> S4c2
  S4c2 --> S4c3
```

| Story | TODOLIST | 依赖 | 可并行 |
|-------|----------|------|--------|
| **S4a** | RG-E8-S4a | 无 | — |
| **S4b-1** | RG-E8-S4b | S4a | — |
| **S4b-2** | RG-E8-S4b | S4b-1 | — |
| **S4b-3** | RG-E8-S4b | S4b-2 | S4b-4（Pack 可后） |
| **S4b-4** | RG-E5 (Deferred) | S4b-2 | S4b-3；不属于 S4 关勾 |
| **S4c-0** | RG-E8-S4c | Garden maintainer acceptance | Diva S4b 可并行 |
| **S4c-1** | RG-E8-S4c | S4b-3 | — |
| **S4c-2** | RG-E8-S4c | S4c-0, S4c-1, S4b-2 | — |
| **S4c-3** | RG-E8-S4c | S4c-2, S4b-1 | 需 Garden API 或 backlog-only |

---

## S4a — Mentle clean-break 删除

> 对应 `11-mentle-deletion-inventory.md` · **无** Laputa 存储依赖 · **P0 最先**

### 目标

从 Diva **产品面** 彻底移除 Mentle/MenPalace/memtle 残留；新增 CI grep gate；**不** 接 MSRV 1.88、**不** 添加 compat adapter。

### 模块 / 文件

| 动作 | 路径 |
|------|------|
| DELETE | `agent-diva-gui/src/components/settings/MentleSettingsCard.vue` |
| EDIT | `agent-diva-gui/src/api/domains/tools.ts` — 移除 Mentle 类型/`listMentleTools` |
| EDIT | `agent-diva-gui/src/api/index.ts` |
| EDIT | `agent-diva-gui/src/api/v1/binding.ts` — 移除 `list_mentle_tools` |
| EDIT | `agent-diva-gui/src/composables/useAppConfig.ts` — 移除 `mentle` 默认块 |
| EDIT | `agent-diva-gui/src/types/toolsConfig.ts` |
| EDIT | `agent-diva-gui/src-tauri/src/legacy/commands.rs` — 移除 `list_mentle_tools` |
| EDIT | `SettingsView.stories.ts`, `NormalMode.test.ts` fixtures |
| EDIT | `docs/research/.../gui-api-governance-inventory.md` — CUT |
| ADD | `just mentle-clean-break-check`（或扩展 `deletion-proof-check`） |

### 禁止

- 删除 sibling `morediva/memtle` 仓库
- 添加 Mentle DB reader / feature flag
- Laputa skeleton 冒充 Mentle recall UX

### 验收 / DoD

| ID | 断言 | 证据 |
|----|------|------|
| S4a-1 | `rg -i memtle Cargo.toml agent-diva-*/Cargo.toml` 零命中 | CI gate |
| S4a-2 | `rg -i mentle agent-diva-gui/src` 零命中（allowlist 无） | CI gate |
| S4a-3 | `binding.ts` 无 `list_mentle_tools` | grep |
| S4a-4 | `cargo check -p agent-diva-gui` 绿 | CI |
| S4a-5 | settings 无 mentle 字段 — unit/story | vitest |
| S4a-6 | TODOLIST `RG-E8-S4a` 勾选 + 指向 inventory | 文档 |

### 回滚

Revert GUI commit；gate 可保留（无害）。

---

## S4b-1 — 轻量存储 + FTS + Schema

> 对应 `04-light-store-candidates.md` · 推荐 **A+B hybrid**

### 目标

在 `agent-diva-state` 新增 profile-local Laputa SQLite adapter；在 `laputa.sqlite3` 引入 `memory_records` + FTS5 + `schema_meta`；Windows FTS5 smoke。新 runtime 只读取 typed schema，旧 registry/blob 只能由离线迁移工具显式转换，**不得**保留兼容读。

### 模块

| Crate/文件 | 职责 |
|------------|------|
| `agent-diva-state/src/laputa_store/` | schema migration, FTS5, CAS row update, record CRUD read |
| `agent-diva-laputa/src/types.rs` | `MemoryRecord`, `MemoryProvenance`, `ReplicationPolicy` |
| `agent-diva-laputa/src/service.rs` | 委托 typed record-store port；不读 registry/blob |
| `agent-diva-local/src/lib.rs` | 打开 store 与现有 StateStore 共存 |

### 契约

```rust
// 示意 — 实现时以 crate 实际 API 为准
trait LaputaRecordStore {
    fn get(&self, memory_id: &str) -> Result<MemoryRecord>;
    fn search(&self, query: &SearchQuery) -> Result<Vec<ScoredRecord>>;
    fn migrate_cas(&self, expected: u64, record: MemoryRecord, outbox: Option<SyncEvent>) -> Result<()>;
}
```

### 测试

| 测试 | 断言 |
|------|------|
| `fts5_available` | CREATE VIRTUAL TABLE … USING fts5 三平台 |
| `cas_concurrent_migrate` | 双 task 一者 Conflict |
| `outbox_crash_safety` | kill -9 后 pending outbox ≥1 且 record committed |
| `schema_version_fail_closed` | 未知 version 拒绝 open |
| `msrv_1_80` | `cargo +1.80.0 check -p agent-diva-state` |
| `single_rusqlite` | `cargo tree -i rusqlite` 单链 |

### 质量门

- [ ] OD-1 Windows FTS5 实测（失败 → fallback B 文档化）
- [ ] 10k records insert + search P95 < 200ms（debug 放宽）

### 回滚

发布前发现问题时回滚整个提交；不得用 feature flag、dual read 或 registry-only fallback 保留旧 runtime 路径。

---

## S4b-2 — PersonaCapsule + L1 + Gateway Tools

> 对应 `06`, `05`, `02`, `12` · **P0 治理收敛**

### 目标

1. `PersonaProjector` + `persona_capsule` 表 + offline Required Context
2. `L1IndexProjector`（128 cap）
3. **所有 mutate** 经 Gateway：`laputa.propose`, `laputa.migrate`, `laputa.retract`
4. CLI/Manager **停止** 直调 `LaputaService` mutate

### 模块

| 模块 | 变更 |
|------|------|
| `agent-diva-execution` | Tool defs + Policy 默认 |
| `agent-diva-local/tool_host.rs` | `LaputaToolAdapter` |
| `agent-diva-state/src/laputa_store/` | persona_capsule, l1_index tables |
| `agent-diva-laputa/provider.rs` | capsule first, sections Optional |
| `agent-diva-cli/main.rs` | laputa 子命令 → ActionRequest |
| `agent-diva-manager-local` | laputa_* → Gateway 编排 |
| `agent-diva-laputa/tests/laputa_gateway_spine.rs` | **新增** |

### 契约（Gateway capabilities）

| capability_id | 效果 | Policy 默认 |
|---------------|------|-------------|
| `laputa.propose` | create proposal | user confirm |
| `laputa.migrate` | apply approved migration | user confirm |
| `laputa.retract` | tombstone | user confirm |

### 测试

| 测试 | 断言 |
|------|------|
| `laputa_gateway_spine` | CLI/Manager mutate 路径零 `LaputaService::` 直调 |
| `persona_offline_boot` | 无 Garden → Required identity in Context |
| `persona_capsule_rebuild` | delete capsule → rebuild from LocalCore |
| `l1_cap_128` | 129th record → LRU eviction logged |
| `identity_migrate_manual` | auto-migrate identity 无 `--yes` → deny |
| 现有 `laputa_smoke.rs` | 仍绿（经 Gateway 路径） |

### 禁止

- `LaputaService` 内嵌 Policy/Authorization
- Manager handler `SqliteStateStore::open` mutate

### 回滚

发布前发现问题时回滚整个提交；**不得**回退到无 Receipt 直写，也不得用 feature flag 并行保留旧路径。

---

## S4b-3 — 本地检索 + Context Merge + Status API

> 对应 `09`, `05` · 依赖 PersonaCapsule

### 目标

1. `CompositeRetriever`（FTS primary, embedding optional feature）
2. `ContextMergePipeline` — query≠replication 强制
3. `GET /v1/laputa/status` + Manager `laputa_status` query
4. `laputa_search` read-only query

### 模块

| 模块 | 变更 |
|------|------|
| `agent-diva-state/src/laputa_store/` | search API |
| `agent-diva-local/` | `GardenContextClient` stub（offline no-op） |
| `agent-diva-session` context | merge hook（或新 `agent-diva-laputa` merge 模块） |
| `agent-diva-manager` | dto/ports/routers status + search |
| `agent-diva-gui` | PersonaMemoryView 绑定 status（partial） |

### 测试

| 测试 | 断言 |
|------|------|
| `retrieval_fixtures` | fix-01..04 top-3 ≥80% |
| `merge_no_persist` | garden_cites 路径零 sqlite write |
| `offline_degraded` | RemoteOnly query → degraded + 0 durable |
| `status_honest` | Garden down → `retrieval=local_only`, 非 fake synced |
| `grep_context_laputa` | `agent-diva-context` 不 import laputa |

### DoD

- [ ] `laputa_status` 返回 garden/persona/sync/retrieval 四维
- [ ] GUI skeleton `laputa_get_section` 至少 bound read path

---

## S4b-4 — Memory Pack import/export（Deferred，非 S4）

> 对应 `10` · 可与 S4b-3 并行（依赖 S4b-2 Gateway）

### 目标

这是 `RG-E5 Migration` 的后续离线能力，不属于 `RG-E8-S4` 关勾范围。本研究仅保留协议草案；不得因 S4 重新激活迁移实现。

### 模块

| 模块 | 变更 |
|------|------|
| `agent-diva-migrate` | Pack transform / verify |
| `agent-diva-cli` | `laputa export-pack` / `import-pack` |
| Gateway | `laputa.export_pack`, `laputa.import_pack` capabilities |
| `docs/logs/` | 证据目录 `rg-e8-s4b-memory-pack/` |

### 测试

| 测试 | 断言 |
|------|------|
| `pack_digest_tamper` | 改 checksum → fail |
| `pack_staging_rollback` | mid-import fatal → production untouched |
| `pack_identity_manual` | identity record 无 confirm → 不 swap |
| `pack_secret_strip` | export 无 secret kind |
| `legacy_fr502_map` | SOUL.md → identity/persona_identity |

### 禁止

- runtime 启动扫描 Mentle DB
- import 自动 enable Garden sync

---

## S4c-1 — Garden resolve 客户端（Context only）

> 对应 `03`, `09` · Connected 只读第一阶段

### 目标

`GardenContextClient`：`GET /health`, `POST /v1/context/resolve|bootstrap`；merge 层 ephemeral；配置 `laputa.sync.v1` profile-local。

### 模块

| 模块 | 变更 |
|------|------|
| `agent-diva-local/garden_client.rs` | HTTP client, timeout, degraded |
| `agent-diva-local` config | garden URL, tenant bind skeleton |
| merge pipeline | wire garden citations |

### 测试

| 测试 | 断言 |
|------|------|
| `garden_resolve_mock` | mock server → Context blocks, 无 sqlite insert |
| `garden_health_degraded` | 503 → local_only banner data |
| `resolve_timeout` | 30s → fallback local |

### 禁止

- resolve JSON 写入 `memory_records`
- 假设非 loopback 多租户隔离

---

## S4c-0 — Garden `laputa-sync/1` 合同冻结

### 目标

由 Garden 维护者在 Garden 仓库接受并测试 `laputa-sync/1`：显式 instance/tenant bind、push/pull cursor、batch idempotency、ack/reject/conflict、tombstone feed 和认证边界。Diva 不得用现有 `GET /v1/memories` 或 section LWW API 冒充该合同。

### DoD

- [ ] Garden 提供版本化、可复现的 HTTP contract fixtures。
- [ ] Garden 端具备 tombstone 与 per-record conflict 语义；不是 section 文件 LWW。
- [ ] Garden 的非-loopback 认证 ADR 已接受；否则 Diva 仅允许 loopback。

## S4c-2 — Selective replication client + ID mapping

> 对应 `03`, `08`, `07` · 需 Garden list cursor 或 interim section GET

### 目标

1. `garden_cursor` + `memory_id` ↔ `mem_*` 映射表
2. 仅调用 S4c-0 冻结的 `laputa-sync/1` pull feed；不得用现有 section 或 memories API 作为同步替代。
3. Inbox apply 经 Gateway + policy filter
4. Persona conflict 检测 → Conflict state

### 模块

| 模块 | 变更 |
|------|------|
| `agent-diva-state/src/laputa_store/` | sync_inbox, id_map, processed_batches |
| `agent-diva-local` | pull worker |
| Manager | `laputa_sync_pull`, conflict query |

### 测试

| 测试 | 断言 |
|------|------|
| `inbox_idempotent` | 同 batch_id 二次 apply → AlreadyApplied |
| `policy_remote_only_skip` | inferred → 不 persist |
| `persona_conflict` | identity LWW 模拟 → Conflict, 非 silent merge |
| `tombstone_pull` | garden retract → local tombstone |

### 开放依赖

- OD-3 Garden `laputa-sync/1` — 未就绪时 S4c-2/3 不开工；只允许 S4c-1 的 read-only Context resolve

---

## S4c-3 — Outbox push + Sync 状态机

> 对应 `08`, `12` · **需 Garden push API 或 backlog-only 模式**

### 目标

1. `sync_outbox` flush worker；exponential backoff + dead letter
2. `laputa-sync/1` batch push（Garden 就绪）或 **persistent backlog**（未就绪）
3. Full sync FSM：LocalOnly → Connected → InSync | Backlog | Degraded | Conflict
4. Manager `laputa_sync_push`, `laputa_sync_conflicts_resolve`
5. GUI sync status + conflict banner

### 模块

| 模块 | 变更 |
|------|------|
| `agent-diva-local` | sync worker, reconnect |
| `agent-diva-state/src/laputa_store/` | outbox state machine |
| Manager/GUI | sync commands + UI |

### 测试

| 测试 | 断言 |
|------|------|
| `outbox_retry_backoff` | 失败 → inflight→pending, 不丢 seq |
| `push_idempotent` | 同 batch 重试 → Garden ack 幂等 |
| `lineage_hop_cap` | hop>8 → reject + audit |
| `offline_edit_reconnect` | 本地改 + garden pull → conflict 或 merge |
| `no_fake_sync` | Garden down → outbox_pending>0, 非 synced=true |

### DoD（S4c 整体）

- [ ] E2E：local propose LocalCore → outbox → (mock) garden ack → inbox persona refresh
- [ ] `GET /v1/laputa/status` sync 维 honest
- [ ] TODOLIST `RG-E8-S4` / `RG-E8-S4c` 可勾选（含证据 log）

### 回滚

Disable garden config → LocalOnly FSM；outbox 保留 backlog；**不** 删 local records。

---

## 跨 Story 质量门（Quality Gates）

| Gate | 命令 | 适用 Story |
|------|------|------------|
| MSRV 1.80 | `cargo +1.80.0 check --workspace` | 全部 |
| 单 rusqlite 链 | `cargo tree -i rusqlite` | S4b+ |
| Mentle clean | `just mentle-clean-break-check` | S4a+ |
| Gateway spine | `laputa_gateway_spine` test | S4b-2+ |
| Context firewall | `rg agent-diva-laputa agent-diva-context/src` 无 import | S4b+ |
| deletion-proof | `just deletion-proof-check` | 全部 |
| clean-break boundary | `just clean-break-boundary-check` | 全部 |
| CI green | `just ci` | 每 Story PR |

---

## 整体验收 DoD（RG-E8-S4 关勾条件）

| # | 条件 | 证据 |
|---|------|------|
| D1 | S4a–S4c 全部 Story DoD 满足 | TODOLIST 勾选 |
| D2 | 无 Mentle 产品面；grep gate 绿 | S4a |
| D3 | 本地 FTS 检索 + PersonaCapsule offline chat | S4b-3 integration |
| D4 | Gateway-only mutate；Receipt/Audit 可追踪 | S4b-2 test + audit log |
| D5 | Garden resolve Context only；无 auto-persist | S4c-1 merge test |
| D6 | Sync backlog honest；Conflict UI | S4c-3 status API + GUI |
| D7 | 旧数据迁移 / Memory Pack | **Deferred to RG-E5**；不属于 S4 验收 |
| D8 | 证据目录 `docs/logs/2026-07-e8-s4-laputa-diva/` | summary + smoke |
| D9 | synthesis §22 开放决策已签收或显式 defer 文档化 | stakeholder |
| D10 | **不** 声称 Garden protocol release-frozen 除非 Garden 确认 | 文档附注 |

---

## 与后置项关系

| 项 | 关系 |
|----|------|
| **RG-E8-S3b** Evolution memory apply | **阻塞于** S4b-2 Gateway + record model；不得绑定旧 section/Mentle |
| **RG-E5** Migration cutover | Deferred；Pack 为 optional offline 路径，非 E5 自动激活 |
| **RG-EX-S4** autonomy | 默认并行线；不抢 S4b Gateway 收敛 |
| **Embedding feature** | S4b-3 后 optional slice；非 S4 关勾门禁 |

---

## 禁止事项（全 Story 通用）

1. 研究结论未 Keep 前编码（已 Keep — 本 outline）
2. 恢复 Mentle 依赖或 MSRV 1.88
3. Markdown 在线 authority / `.laputa/` 双写
4. 第二 ToolExecutionGateway
5. 14 section 全量 mirror
6. Garden resolve → sqlite INSERT
7. fake sync success / `Ok([])` 成功剧场
8. 无 HITL 的 identity Pack import

---

*Outline 版本：2026-07-24 · 链接 synthesis §20 · TODOLIST RG-E8-S4*
