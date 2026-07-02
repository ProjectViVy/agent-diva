# Laputa PRD v0.0.4 — Edge Case Review (Reviewer Gate)

审阅范围: prd-laputa-2026-06-12/prd.md (32 FRs, 7 段) + .decision-log.md (D-001~D-009)
审阅目的: bmad-prd Finalize Step 3 Reviewer Gate — 找分支/边界条件
审阅时间: 2026-06-12
审阅人: edge case hunter (子 agent)

---

## 1. Summary

PRD 在 32 FRs 上覆盖了"写/读/事件/路由/迁移/边界/NFR"全栈, 治理 4 道 (review/changelog/audit/rollback) 设计完整, 14 → 1 稀薄层 + 5 TBD 显式 status 标记符合 D-005/D-007 决策。但 FR-1xx/FR-5xx 大量边界条件未写明具体行为, 实施时大概率出现以下 critical/high 风险: **(a) 写半中间态原子性** (FR-103 changelog 已落 + audit 失败的兜底) **(b) flock 跨进程/死锁** (FR-104/FR-704 在 Windows + 多 Laputa 实例下未明确) **(c) TBD sections 写入行为** (FR-102/FR-501 拒收 #10-14, 但 FR-201/FR-202 读时返 status=tbd — 写/读语义不一致) **(d) 30 天撤销窗口边界** (FR-403/FR-703 缺毫秒/时区/已撤销的级联) **(e) 迁移原子性** (FR-503 写到一半崩了) **(f) schema 升级并发** (FR-506 多进程同时升)。建议补 §5.1/5.5/5.6/5.7 的"边界条件"子段, 给每条 FR 显式写 3-5 条 boundary case + 期望行为, 再走 polish。

---

## 2. Edge Case Table

| FR id | 边界条件 | 现状 (PRD 描述) | 建议 |
|-------|----------|----------------|------|
| **FR-101** | 0 字段空 proposal (proposal_type="", content=null) | 未写 | 入口 MUST 校验 EvolutionProposal 必填字段, 缺字段返 `Err::InvalidProposal`, 走 FR-705 7 种错误枚举需补第 8 种 `InvalidProposal` |
| **FR-102** | 8 种 proposal_type 之外的 `sop_create` 路由到 #1 sub-section, 但 #1 满了 (5kb) | FR-501 max_size_kb=5kb, FR-102 未明 | 写前 MUST 检 section 大小, 超 max 返 `Err::SectionSizeExceeded`; sop_create 落 #1 sub-section 时 MUST 显式命名 `sop_{iso8601}` 防冲突 |
| **FR-103** | atomic 失败: changelog 已写盘 + audit 事件发出失败 (网络断) | 4 道串联任一失败整体回滚, 但"回滚"指什么未明 | MUST 区分 (a) changelog 写盘前失败 → 整体放弃, (b) changelog 写盘后 audit 失败 → 标 changelog `pending_audit=true`, 后台 retry 队列, 重试 3 次后转 needs_attention; 不要"假装回滚" |
| **FR-104** | flock 锁死 (进程崩溃持锁) + Windows msvc 行为差异 | flock + deadlock detection 5s timeout 写在 FR-704 | MUST 加 stale lock recovery: 写锁附带 owner_pid + acquired_at, 5s timeout 后检 PID 是否存活, 不存活则强夺; Linux 默认 flock 跨 fork 行为需测 |
| **FR-201** | 0 section 全新装 (`.laputa/state.json` 不存在) | FR-503 已答"创建空骨架", 但 FR-201 读时行为未明 | GET snapshot 在文件不存在时 MUST 返 200 + 全 14 section (9 owned 空对象 + 5 TBD status=tbd), 不得 500/404; 加 `laputa_initialized: false` 标志给 UI 引导首次 onboarding |
| **FR-202** | 未知 section name + path traversal (`../../etc/passwd`) | 仅说"未知 name 返 404" | MUST 白名单 14 个合法 name, 非法字符 `/`, `..`, `\0` 一律返 400; `name` 字段 MUST 正则 `^[a-z_]+$` |
| **FR-301** | SSE 客户端断网 30min 后恢复 + 期间 1000 个 proposal 事件堆积 | SSE + 60s 轮询 fallback 写在 FR-301 | SSE MUST 带 `Last-Event-ID` resume 协议; 客户端重连带 last-event-id, server replay 缺失事件; 服务端事件缓冲 MUST 设上限 (e.g. 10000 条 ring buffer), 超出后强制 client 走 snapshot since= 重拉, 避免内存爆炸 |
| **FR-401** | 1M changelog 分页 page=50000 page_size=100, 总响应时间 | 仅说 default 20, max 100 | MUST 加 `page * page_size ≤ 100000` 上限, 超返 400; 加 cursor-based 分页 (除 offset) 给 UI 无限滚动; 1M 数据下 MUST 有 changelog target_section 索引 (B-tree on section+created_at) 否则全表扫 |
| **FR-403** | 30 天边界 30天 0:00:00 vs 30天 23:59:59 vs 31天 0:00:01; 已 rollback 的 changelog 再次 rollback; rollback 写到一半崩了 | 写明 30 天窗口外返 Err::RollbackExpired | MUST 用 server 端权威时钟 (非 proposal.created_at) 计算 30×86400 秒; 二次 rollback 返 `Err::AlreadyReverted`; rollback 中断 (changelog 标 reverted=true 但 target_section 未还原) MUST 启动时扫 `pending_revert` 标记自动续完 |
| **FR-501** | 14 section 全 TBD (1-9 owned 全空) + 写超大内容 (memory_md 30 行 ≤2kb, 实际 1MB) | max_size_kb + content_type 枚举已写 | TBD 5 section 的 `content_type=tbd` MUST 显式接收任意 JSON/markdown/binary 都不拒, 但 size 上限仍按 100kb 执行; 写超大 MUST 在 write 入口序列化前检, 拒收返 `Err::SectionSizeExceeded` (FR-102 同款) |
| **FR-503** | 迁移中崩了 (旧 8 文件已备份到 legacy/ 但 1 稀薄层只写 3 个 section) | "旧文件保留 1 release" 写在 FR-504, FR-503 未明原子性 | MUST 写 transaction log `.laputa/migration.lock` + 每 section 写完 fsync 一次, 崩了重启 MUST 检 lock 续迁; legacy/ 备份 MUST 校验大小 > 0 + sha256, 备份失败回滚整体迁移 (拒绝启动 v0.0.5) |
| **FR-506** | schema_version 1.0.0 升 2.0.0 + 2 个 Laputa 进程同时升 (race) | 写明"不兼容走 schema_upgrade changelog" | MUST 升级加全局分布式锁 (advisory lock on `.laputa/schema.lock`), 2 进程同时升 2nd MUST 等 1st 完; 降级 (2.0.0 → 1.0.0) MUST 显式拒绝返 `Err::SchemaDowngrade`; 升级前 MUST 备份 state.json 到 `.laputa/legacy/{iso}/state.json` |
| **FR-601** | Mentle 跨进程调 LaputaWrite (经 HTTP) 旁路 caller 检查 | "入口处检查 caller" 未明确跨进程 | MUST caller 验证不能只信 in-process 函数调用栈, 走 HTTP 时 MUST 加 signed token (e.g. HMAC over pid+timestamp), Laputa 端验签失败返 `Err::Unauthorized`; token 5s 过期防重放 |
| **FR-603** | mentle_recall_policy 字段值 = "always" (v0.0.3 旧 enum 残留) 或字段缺失/非 string | 字段类型已锁 off/on_demand, 删 always 在 D-009 | MUST 启动时严格 schema 校验, 非法值返 `Err::SchemaIncompatible` 触发 FR-506 升级流程; 字段缺失默认 on_demand, 但 MUST 写 1 次 migration 显式补默认值 |
| **FR-701** | 性能指标 mock 100 行达标, 实接 1M changelog 退化 | 仅给 mock 数据目标 | MUST 补 benchmark 套件: 1M changelog 读 list (≤500ms), 1M 写后 snapshot (≤500ms), snapshot 14 section 全 (≤200ms); CI 加 perf regression gate, 退 20% 阻 PR |
| **FR-704** | 100 并发同 section 写, 全部走 flock, 排队 5s 超时 | deadlock detection 5s 写在 NFR | MUST 区分 (a) deadlock = 互等, (b) 长队列 = 单进程慢; timeout 必报 needs_attention 事件 (复用 FR-303), 同时 metrics `laputa_lock_wait_seconds` histogram 暴露; 同 section 高并发场景 MUST 给客户端返 `Retry-After` 头 |
| **FR-705** | 7 种错误枚举, 错误嵌套 (ConflictUnresolved 内部因 SchemaIncompatible) | 7 种枚举已列 | MUST 定义错误链 `WriteError::ConflictUnresolved { source: Box<SchemaIncompatible> }`, thiserror #[source] 透传; 客户端 MUST 能 match 到根因, 不要把所有错误拍平成 "internal error" |
| **FR-706** | Prometheus scrape 失败 + metrics 端点本身挂 | 3 metrics 列出 | MUST metrics 端点独立线程池, 主写路径不影响; scrape 失败 MUST 自身不 crash, 返空 200; 加 `laputa_metrics_scrape_errors_total` 自监控 |
| **跨 FR 权限** | 谁能写 #1 identity (Laputa 入口必经, 但 selfinprove user 审后谁触发) | review 写在 FR-103 | MUST 显式画权限矩阵: identity = `agent_self` (FR-501), 实际写触发者 = selfinprove proposal_inbox user-approved; 谁能 rollback 别人的 changelog: 必须同 user 或 admin role, 跨 user rollback 返 `Err::Unauthorized` (FR-705 枚举需补 `PermissionDenied` 第 9 种) |
| **跨 FR 降级** | Laputa API 挂了, UI / Mentle recall / AutoDream Orient 行为 | 未写降级契约 | MUST 显式降级矩阵: (a) UI 读 snapshot 失败 → 走 localStorage 缓存 + 标 `laputa_offline=true`; (b) Mentle recall 失败 → 静默退化 (不注入 context, 记 metrics); (c) AutoDream Orient 读失败 → 跳过本轮 Orient, 标 degraded; (d) 写失败 → 写入 `pending.jsonl` (per D-008 OQ #7) 队列, 启动时回灌 |
| **跨 FR 大小** | snapshot 1GB (14 section 全 1MB+ changelog 全量嵌入) | max_size_kb 上限在 FR-501, 但 snapshot 端未明 | MUST snapshot 返 SectionSummary (metadata + 截断的 content preview), 不返全文; 全文 MUST 走 per-section GET (FR-202); snapshot 加 `content_truncated: true` 标志 |
| **跨 FR 嵌套** | proposal 嵌套 100 层 (proposal.content 引用另一个 proposal) | FR-102 路由未限深 | MUST proposal 反序列化时检嵌套深度, 超过 10 层 (or configurable) 拒收返 `Err::ProposalTooDeep`; 防 JSON bomb DoS |

---

## 3. Verdict

**Verdict: NEEDS WORK** (非 blocker, polish 阶段必补)

**Top 5 critical/high findings (按修复优先级):**
1. **[CRITICAL] FR-103 atomic 半写**: changelog 落盘后 audit 失败的兜底未明, 实施时会出"假装回滚"导致数据/审计不一致
2. **[CRITICAL] FR-503 迁移原子性**: 写到一半崩了 + legacy 备份失败场景无事务保护, 升级用户可能丢数据
3. **[HIGH] FR-104/FR-704 flock 跨进程**: Windows + 多实例 Laputa (dev/staging 同时跑) 行为未测, 死锁恢复策略缺失
4. **[HIGH] FR-403 30 天撤销**: 毫秒/时区/已撤销级联/中断恢复 4 边界未明, 实施 1 处错就出"假成功"
5. **[HIGH] FR-301 SSE 断网 resume**: Last-Event-ID 协议 + 服务端 ring buffer 上限未明, 长断网后内存爆

**额外 follow-up** (建议转 Laputa 架构后续 PRD): 降级矩阵 (4 种失败 × 3 种消费方 = 12 case) / 权限矩阵 / 大小边界 (snapshot 1GB) / 嵌套 proposal DoS / TBD 写读语义不一致。

**未发现 P0 blocker**, 32 FRs 整体可走 polish 阶段补 §5.1/5.5/5.7 的"边界条件"子段, 不需重写。
