# 12 — 安全、治理与失败模型

> **RG-RSCH-08e** · 对应研究问题 **Q23、Q24（部分）** + ADR authority spine
> 日期：2026-07-24

## 1. 威胁模型

### 1.1 STRIDE 摘要

| 威胁 | 场景 | 严重性 |
|------|------|--------|
| **Spoofing** | 伪造 Garden 下行 batch / 恶意 Pack | 高 |
| **Tampering** | 绕过 proposal 直写 sqlite；UI raw invoke | 高 |
| **Repudiation** | 记忆变更无 Receipt/Audit | 中 |
| **Info Disclosure** | secret 同步到 Garden；Pack 泄漏 token | 高 |
| **DoS** | 超大 Pack / sync batch / FTS 查询 | 中 |
| **Elevation** | 未授权 identity_patch；Manager 开 DB 写 | 高 |

**攻击面**：本地 profile 文件、Garden HTTP、Memory Pack import、Evolution proposal、GUI settings、sync outbox replay。

### 1.2 关键场景与缓解

| 场景 | 影响 | 缓解 |
|------|------|------|
| 攻击者替换 `laputa.sqlite3` | 人格/记忆篡改 | profile 路径权限；启动 checksum spot-check；CorruptState fail-closed |
| 中间人 Garden HTTP | 假 replication | TLS +（未来）batch HMAC；inbox policy filter |
| 恶意 Pack import | 注入 identity | digest verify；identity manual approve；staging rollback |
| Agent 工具滥用 | 未授权 migrate | Gateway Policy deny；capability allowlist |
| 检索→持久化 | 污染 persona | merge 层无 persist hook（`09`） |
| Sync 回环 | 数据膨胀/振荡 | lineage hop cap（`08`） |

## 2. 治理 spine（当前代码事实）

```text
Request → Context → Plan (opt) → Policy → Authorization → ToolExecution → Receipt → Audit
```

| 组件 | 角色 | 证据 |
|------|------|------|
| `ToolExecutionGateway` | 唯一 effectful 执行 | `agent-diva-execution/src/lib.rs:1-5` |
| Gateway 流程 | Policy → Authorization → Tool → Receipt + Audit | `gateway.rs:209-300` |
| `LaputaService` | domain CAS；**非** Tool 路径 | `clean-break-r7-laputa.md:20` |
| `LaputaMemoryProvider` | 只读 Context | `provider.rs:8-9` |
| Proposal/Migration ports | 无 StateStore | `laputa/lib.rs:3-6`, `types.rs:43-51` |
| Migrate binary | offline-only；runtime 不依赖 | `agent-diva-migrate/lib.rs:1-5` |

**Embedded Laputa 扩展原则**：所有 **mutate memory** 的 product 路径经 Gateway Tool adapter；`LaputaService` 仅被 adapter 调用。Manager HTTP **不** `SqliteStateStore::open` 做写入。

### 2.1 建议 Tool capabilities（新）

| capability_id | 效果 | Policy 默认 |
|---------------|------|-------------|
| `laputa.propose` | 创建 proposal | user confirm |
| `laputa.migrate` | 应用 approved migration | user confirm |
| `laputa.retract` | tombstone record | user confirm |
| `laputa.sync.push` | flush outbox | auto if configured |
| `laputa.import_pack` | staging import | manual + `--yes` |
| `laputa.export_pack` | 写 Pack 到路径 | user confirm |

Evolution memory target 应映射到 `laputa.propose`，**非**独立写路径（Q29 交叉）。

## 3. Q23 — 哪些数据默认不得离开本地

| 数据类 | 默认策略 | 机制 |
|--------|----------|------|
| API keys / tokens | **never push / never export** | `kind=secret` |
| 会话 working checkpoint | **never push** | Context retention；无 sync hook |
| 未 accept proposal | **never push** | `ProposalState != Applied` |
| Session raw transcript | **never push bulk** | SessionStore；可选 salient summary |
| `sensitivity=high` PII | **never push** unless profile opt-in | policy tag |
| Ephemeral Garden query hits | **never persist** | Context only（`09`） |
| Inferred personality | **never LocalCore** | RemoteOnly |
| Local journal (#10) | **default LocalOnly** | 矩阵 `07` |
| Device/instance secrets | **never export** | Pack redaction |
| Execution journal / HITL state | **never sync** | 独立 sqlite |

**默认允许上行**（ReplicationPolicy + operator config）：LocalCore 非 secret persona 字段、accepted non-sensitive facts/preferences、tombstone retract 事件。

## 4. 隐私与 local-only 数据

### 4.1 Profile 隔离

- 所有 DB 在 `profiles/<id>/` — `agent-diva-local/src/lib.rs:84-108`
- Laputa `memory_for` 不跨 profile — `service.rs:266-282`
- CLI smoke 证明跨 profile 不可见 — `agent-diva-cli/tests/laputa_smoke.rs`

### 4.2 Session scope

- `MemoryScope::Session` 仅同 session Context — `service.rs:275-279`
- Session memory **默认不上行**（handoff raw sessions excluded）

### 4.3 加密 at rest（开放）

- 当前 `SqliteStateStore` **无** SQLCipher — `state_store.rs`
- 假设：OS 磁盘加密；未来可选 profile encryption key（stakeholder 决策）

### 4.4 日志与 telemetry

- Audit/Receipt：**仅** id + digest + principal
- **禁止** info 级别 log 全量 memory content
- GUI 设置页不显示 secret 值（仅 env 名）

## 5. Gateway-only writes

### 5.1 允许路径

```text
Operator / Agent
  → ActionRequest (laputa.*)
  → Policy.evaluate → AuthorizationProvider.authorize
  → ToolExecutionGateway.execute
  → LaputaToolAdapter → LaputaService / sync inbox apply
  → Receipt + AuditRecord
```

### 5.2 禁止路径

| 路径 | 违反 |
|------|------|
| GUI `invoke('write_laputa')` 直写 | AGENTS.md API surface |
| Manager handler 打开 sqlite mutate | manager-local 边界 |
| Model tool 无 authorization | authority spine |
| Garden inbox bypass tombstone/policy | sync apply 必须 filter |
| `RetentionContextProvider` 写 Laputa | 组件隔离 |
| Import 直 SQL INSERT 无 proposal | governance |

### 5.3 Read 路径（无需 Gateway）

- `LaputaMemoryProvider.provide`
- Local FTS search（read-only connection）
- Garden HTTP resolve（read）
- Manager GET section / status

## 6. 诚实降级（Q24 安全视角）

| 条件 | 诚实行为 | 禁止 |
|------|----------|------|
| Garden down | `retrieval=local_only`, sync backlog | 假装 synced |
| Persona conflict | block auto-merge | LWW silent |
| Import preview fail | abort staging | partial apply |
| RemoteOnly miss offline | 明示不可用 | Session summary 冒充 LT memory |
| Store corrupt | `LaputaError::CorruptState` | 继续用脏数据 |
| FTS index bad | search error + rebuild offer | 静默空结果 |
| Capability unbound | `not_bound` / `degraded` | `Ok([])` 成功剧场 |

Status：`GET /v1/laputa/status`（`09`）；stub 期 GUI binding 必须 honest stub（`gui-api-governance-inventory.md` §8.1）。

## 7. 失败模式与恢复（FMEA）

| ID | 失败 | 检测 | 严重度 | 恢复 | 用户可见 |
|----|------|------|--------|------|----------|
| F1 | CAS conflict | `LaputaError::Conflict` | 低 | retry propose | toast retry |
| F2 | Crash mid-commit | outbox pending + record | 高 | replay idempotent | sync backlog |
| F3 | FTS corrupt | query error | 中 | rebuild from records | degraded search |
| F4 | Garden 401/403 | HTTP status | 中 | stop push | banner auth |
| F5 | Sync loop | hop_count | 高 | drop batch + audit | support id |
| F6 | Tombstone revive | revision check | 高 | reject + alert | conflict UI |
| F7 | Pack import half | staging marker | 高 | rollback staging | import failed |
| F8 | Persona conflict | merge fail | 高 | manual resolve | banner |
| F9 | Outbox dead | 24h fail | 中 | manual replay | settings |
| F10 | Registry corrupt | validate fail | 高 | fail-closed | error screen |

### 7.1 数据完整性不变量

1. `content_digest` 变 → `revision` 必增
2. `tombstone=true` → capsule 不含该 id
3. outbox seq 单调；acked 不可丢
4. Pack `manifest.content_digest` 覆盖所有 records
5. sync `batch_id` 唯一 processed

## 8. Authorization 建议

| 操作 | 默认授权 |
|------|----------|
| Local propose (non-identity) | CLI `--yes` / GUI HITL |
| Persona identity migrate | **manual always** |
| Sync push secrets | Policy deny |
| Pack import identity | manual |
| Garden replication apply | auto if policy + monotonic revision |
| Retract/tombstone | user or Garden governance |
| Auto compression migrate | deny for LocalCore kinds |

Evolution `identity_patch` 已要求 Laputa decide — 保持一致（`service.rs:121-156`）。

## 9. Observability

| 事件 | Sink | 字段 |
|------|------|------|
| memory migrated | Audit + Receipt | section, revision, proposal_digest |
| sync batch applied/rejected | Audit | batch_id, counts |
| conflict opened/resolved | Audit + status API | keys, strategy |
| import pack | Receipt | pack digest, record_count |
| policy deny | Audit fail-closed | capability_id, reason |
| persona_capsule rebuilt | Audit | capsule_revision, digest |

**Redaction**：日志与 GUI 仅显示 digest 前缀。

## 10. Clean-break 与 Mentle 删除的安全含义

| 项 | 安全收益 |
|----|----------|
| 删除 Mentle 依赖 | 缩小 supply chain；无 MSRV 1.88 C 栈 |
| 禁止 Mentle DB reader | 消除 legacy 解析攻击面 |
| 无 online legacy import | 防止恶意旧文件自动执行 |
| `deletion-proof-check` | CI 阻止 compat route 回归 |
| 无 dual Gateway | 单授权路径可审计 |

用户旧 Mentle 数据：**仅** offline migrate → Pack → proposal；不可 runtime 链接。

## 11. 候选控制模型

| 模型 | 优点 | 缺点 |
|------|------|------|
| **A. Gateway + Policy tags**（推荐） | 统一 spine；Receipt 证明 | 需新 tool defs |
| B. LaputaService 内嵌 auth | 少 hop | **绕过 Gateway — 否决** |
| C. Garden 全权 | 简单 | 离线不可用 |
| D. 文件 ACL only | 简单 | 无 agent 路径治理 |

## 12. 推荐结论

1. **Default-deny push** for secrets, sessions, proposals, inferred, execution state。
2. **All mutates via Gateway** with Receipt/Audit；LaputaService 仅 adapter 可写。
3. **Honest degradation** 是安全特性 — 不伪造 sync/检索/空成功。
4. **Fail-closed** on corrupt policy/import/sync/persona anomalies。
5. **FMEA F2/F6/F7** 为 P0 测试矩阵项。

## 13. 被否决方案

- Mentle recall 写 Laputa — PRD FR-106 旧方向，Mentle deleted
- Trust Garden TLS alone without batch attestation — 待 Garden auth 冻结
- Silent import of Garden resolve — pollution + spoofing
- `Ok([])` capability stub 冒充已同步 — GUI governance 禁止
- LaputaService 公开给 GUI invoke — API surface 违规

## 14. 开放假设

- [ ] Garden mTLS / HMAC batch signing 时间表
- [ ] Profile at-rest encryption 是否 in-scope E8-S4
- [ ] GUI sensitivity 标签与 push 预览
- [ ] 是否需 `laputa.audit.read` 独立 capability

## 15. 研究问题汇总

| Q | 要点 |
|---|------|
| Q23 | secret/session/proposal/inferred/transcript 默认不出本地；`07`+Policy tags |
| Q24 | degraded=能力受限+明示状态；永不 block chat；禁止 fake sync |

## 16. 验收门禁（交叉 implementation-outline）

| 门禁 | 命令/测试 |
|------|-----------|
| 无组件 bypass Gateway 写 | grep + integration `plan-execution/tests/authority_spine.rs` 模式扩展 |
| 无 Mentle runtime | `just deletion-proof-check` |
| 无 legacy laputa routes | `just clean-break-boundary-check` |
| Import fail-closed | migrate/pack test half-fail rollback |
| Query 不持久化 | merge layer unit test 无 sqlite write |
