# Summary — GA-MEM-PARITY Wave 4（AutoDream 去重 — G4）

- 版本：`v0.0.8-wave4-autodream-dedup`
- 日期：2026-08-06
- 类型：AutoDream 双写边界收口（2 切片 + 1 docs，每切片独立提交）
- 前置：`v0.0.7-wave3-read-closure`

## 做了什么

Wave 4 只做 inventory §10.3 明确点名的 **G4 双写边界未定**问题：

> `memory_add` 即时 apply 的内容，AutoDream 反思可能基于 session evidence 重复提出同内容候选。

Wave 0/1/2/3 已修好"写→存→召回→遗忘→prefetch"。G4 是最后一个与已落地
typed authority 直接联通的代码级缺口——其余 Wave 4 项（G1/G2/G3/G5/G6/G7
/G10/G11/G12）都是端到端/产品级/真机验收，归 G2D+ 桌面验收或后续独立 Wave。

### S1 laputa：`LaputaService::applied_authority_digests` + 错误变体 — `f042bba4`

- `agent-diva-laputa/src/service.rs`：新增 async
  `LaputaService::applied_authority_digests()`，内部通过
  `TypedMemoryStore::open_existing_canonical(workspace_root)` 打开既有
  typed authority store，列出全部记录，按四重条件过滤：
  - `trust == AppliedAuthority`（排除 WorkingMemory / Observed / Inferred 等）
  - `tombstone.is_none()`（排除 tombstone 本身）
  - `scope.session_id.is_none()`（排除 session-scoped 工作 checkpoint）
  - `!superseded_target_ids.contains(&id)`（排除被 supersedes tombstone 指
    向的记录——复用 Wave 3 已验证的 `TypedMemoryStore::superseded_target_ids`）
  返回每条 `record.content` 经
  `agent_diva_core::evolution::memory_candidate_content_digest` 计算后的
  digest 字符串（与 AutoDream `crate::content_digest` 算法完全一致）。
- 失败语义：
  - `TypedMemoryStoreError::InvalidBackup`（typed DB 文件尚未存在，
    首次 run 场景）→ 返回 `Ok(Vec::new())`，graceful no-op
  - 其他（IO / schema / integrity / persistence）→ 映射到新增
    `LaputaError::InvalidState(String)` 并向上返回
- `agent-diva-laputa/src/error.rs`：新增
  `LaputaError::InvalidState(String)` 变体与稳定 API code `invalid_state`
  （覆盖 service-layer wrapper 暴露的 typed-store 内部不变量违规）。
- 测试（`service.rs` 末尾 `mod wave4_tests`，tempdir + 真实 typed store）：
  - `applied_authority_digests_returns_only_active_authority`：写入
    (a) AppliedAuthority 活跃记录 → 包含；(b) session-scoped checkpoint →
    排除；(c) supersedes tombstone 目标 → 排除；(d) tombstone 自身
    （content 空）→ 排除。断言返回长度 = 1。
  - `applied_authority_digests_excludes_superseded_targets`：写入 A →
    before 含 digest；写 tombstone 指向 A → after 不再含 digest。
  - `applied_authority_digests_graceful_missing_store`：LaputaService 直接
    打开空 workspace（从未创建 typed DB）→ 返回空数组，不报错。

### S2 autodream：worker 双路 digest 接入 + 端到端测试 — `a39638bb`

- `agent-diva-autodream/src/worker.rs reflect()`：构造
  `BoundedReflectionInput` 时，`existing_memory_digests` 由原"laputa
  section digest"单路改为双路合并：
  - legacy：`collected.items.iter().filter(|i| i.source == "laputa").map(content_digest)`
    （保留——legacy authority mode 环境仍可读老 section）
  - typed：`self.laputa.applied_authority_digests().await`（新）
  - `HashSet` 并集写入 `existing_memory_digests`
- 失败处理：typed digest 取失败 → `tracing::warn!`（G4 degraded）+
  `typed_digests = vec![]`，不阻断 AutoDream run（AutoDream 不应因 typed
  读失败而拒绝产候选；只是去重变弱——与 Wave 3 legacy prefetch Failed 降级
  策略一致）。
- 测试（`worker.rs` 末尾 `mod wave4_tests`，tempdir + 真实 TypedMemoryStore
  + 真实 LaputaService + CandidateGate）：
  - `candidate_duplicate_against_typed_authority_is_rejected`：
    `memory_add` 一条 AppliedAuthority → 调 `applied_authority_digests()`
    拿到 digest 列表 → 构造同内容候选 → gate 结果
    `rejected.code == Duplicate`。
  - `candidate_fresh_against_typed_authority_is_accepted`：同样 setup 但
    候选 content 完全不同 → gate accepted。
  - `superseded_authority_record_no_longer_blocks_duplicate_candidate`：
    写入 A → before 含 digest；写 tombstone 指向 A → after 不再含 digest；
    同内容候选重新被接受。

### S3 docs 收口 + TODOLIST 更新 — 本次

- `TODOLIST.md`：
  - 状态行推进为"Wave 0/1/2/3/4 已完成（2026-08-06）；Wave 5（巩固与清理）
    待排期"
  - 新增 `WAVE4-AUTODREAM-G4` 子块（3 项已勾选）
  - 新增"Wave 4 延期项"条目块，G1/G2/G3/G5/G6/G7/G10/G11/G12 归 G2D+
    或后续独立 Wave
- `docs/architecture/memory-write-paths-contract.md`：priority rule #1 补
  "Realized in Wave 4" 追溯注脚，指向新 service 方法 + worker 双路接线 +
  CandidateGate Duplicate 拒绝。
- `docs/logs/2026-08-05-memory-ga-parity-inventory/v0.0.8-wave4-autodream-dedup/`
  四件套（summary/verification/release/acceptance）。

## 验证

- 每切片 `cargo fmt --check` + `cargo clippy -p <crate> --all-targets -- -D warnings`
  + `cargo test -p <crate>`。
- 全 workspace `cargo test --workspace`：laputa / autodream / agent / core /
  tools / manager / migration 全绿；CLI 6 个既有 wiremock 502 失败
  （`CLI-WIREMOCK-502-PREEXISTING`）与本迭代无关。

## 影响范围

- laputa：新增 service 方法 + InvalidState 错误变体 + 3 个 wave4_tests。
- autodream：worker.rs reflect() 一处双路合并 + 3 个 wave4_tests。
- docs：TODOLIST 状态 + 延期条目；memory-write-paths-contract.md 追溯注脚；
  v0.0.8 四件套。
- 不引入 GUI / CLI / Channel / Tool 协议变更；不影响现有 AutoDream run 的成功
  率（typed 读失败降级为 warn + 空数组）。

## 决策基线

- inventory §6 Wave 4（AutoDream 完全可用）+ §10.3 G4（双写边界未定）。
- 用户拍板（2026-08-06）：最小闭环，只做 G4 去重；其他 Wave 4 项全部归 G2D+
  桌面验收或后续独立 Wave。
- 不引入 `Suggestion` 新 proposal 变体：复用 `CandidateRejectionCode::Duplicate`
  表达"跳过"语义；`rejected` 数组记录原因；`outputs.rs` 将 accepted 写入
  Laputa proposal。
- 不修改 `suppressed_content_digests`：那是 CandidateSuppressionStore 维护
  的用户主动拒绝 digest，与 typed authority 去重是两个独立维度。
- 不引入新 trait 方法/工具/配置字段：纯接线 + 测试 + 文档。
