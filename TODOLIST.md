# TODOLIST

项目级**活跃**待办与延期项。已完成历史见 Archive Index。

**严重度标签**使用 `sev-P0`（阻断）…`sev-P3`（琐碎），与已归档的
「Plan 阶段 P1/P2/P3」命名无关。

---

- [x] **LAPUTA-CLEAN-BREAK-BML-DOCSTRING**：`just laputa-clean-break-check` 曾命中
      `agent-diva-laputa/src/bml/mod.rs:3` 的退休术语；该行已存在于当前 HEAD
      （`9758ac32`），与 C1c 差异无关，但会使 `just ci` 在完成 fmt/check/test 后最终
      返回 1。应在独立术语清理提交中改为纯 BML 描述，并重跑 clean-break gate；不要
      放宽 `scripts/ci/check_laputa_clean_break.py`。2026-08-11 已改为纯 BML 描述，
      focused clean-break gate 通过。

## Active Plan

当前主线：在 **typed Laputa clean-break（GMH-24）之后**，先完成
**AutoDream → 候选 → Laputa 提案 → typed Memory → Recall 反馈**的产品纵向闭环，
再执行最终真实桌面验收。GMH-00..24 架构底座已完成并归档，但 Evolution 当前仍是
占位/不可用产品，不能以已有页面或提案底座宣称可用。

全量执行顺序、依赖、人工暂停点和项目级完成定义见
[`docs/architecture/todolist-master-execution-plan.md`](docs/architecture/todolist-master-execution-plan.md)。
Codex“目标”功能必须按该蓝图逐切片推进，不得把清单机械并行执行。完整产品闭环计划见
[`docs/dev/autodream-laputa-product-closure/`](docs/dev/autodream-laputa-product-closure/)。

- [x] **OPENHARNESS-BENCHMARK-RESEARCH：OpenHarness 深度调研与 Agent-Diva 演进提案（已决策关闭）** `sev-P1`
  2026-08-10 用户决策：不实施 `agent-diva dry-run` 离线 Pre-flight、Workflow
  Profile 秘钥隔离和 `ohmo` 个人 Agent 应用扩展；原综合事项关闭。仅保留
  EventBus Trait Hook 管道，拆为下方独立延期项。
- [ ] **EVENTBUS-TRAIT-HOOKS：EventBus Trait Hook 管道** `sev-P1`
  来源于 OpenHarness 调研；保留为未来扩展点，但当前明确延期，不进入本轮实施。

- [ ] **HARNESS-GAP-RESEARCH：Claude Code 对比后的 Agent Harness 演进与优化** `sev-P1`
  2026-08-05 完成 Claude Code 与 Agent-Diva 基础 Harness 能力全面调研 (文档见 `morediva/claude-code-vs-agent-diva-harness-research.md`)；确定 4 大演进方向：1) Prompt Cache 结构对齐与前缀保护 2) Plan Mode 物理限制状态机 3) TF-IDF 工具按需索引与延迟挂载 4) Subagent Git Worktree 隔离机制。
  **排期决策（2026-08-10）**：作为当前下一项 SEV-P1 工作推进；实施前按四个方向
  拆分可独立验收的切片与依赖顺序。
  - [x] **CTX-C0-RESEARCH：上下文管理增强定向调研与架构决策**（2026-08-10）
        主报告：
        [`docs/research/context-management-enhancement-2026-08/c0-baseline-and-architecture-decisions.md`](docs/research/context-management-enhancement-2026-08/c0-baseline-and-architecture-decisions.md)。
        冻结 ADR-CTX-0..5；主线 C1 稳定前缀 → C2 类型化预算 → C3 工具结果引用 →
        C4 按需工具/Recall → C5 验收。对照样本：Codex / Claude Code+OpenHarness /
        GenericAgent；Diva 落点 `prepare_runtime_context` / BML / ToolAssembly。
        **本切片仅文档，无代码。**
  - [x] **CTX-C1：稳定前缀与 Prompt Cache 对齐**（主规格：
        [`docs/research/context-management-enhancement-2026-08/claude-code-prompt-cache-alignment.md`](docs/research/context-management-enhancement-2026-08/claude-code-prompt-cache-alignment.md)）
        以 Claude Code 为唯一深读样本；provider 已有 `apply_cache_control`，修装配层 bust。
    - [x] **C1 规格文档**（2026-08-10）：CC 机制 + Diva B1–B9 + P0/T1–T8
    - [x] **C1-0 契约与现状刻画**：新增 provider-neutral
          `ContextSection` / `SectionStability` / `PromptSection` 最小骨架与固定逻辑顺序；
          tests 冻结 time/session 首 system、plan 插入、WM/Recall 顺序及工具 definitions
          集合语义；生产序列化输出未改变。agent lib 390 tests 与工作区三门全绿。
          日志：`docs/logs/2026-08-context-management-enhancement/v0.0.1-c1-0-contract-characterization/`。
    - [x] **C1-CONSTRUCTION-DECISIONS：实施前决策修订**（2026-08-10）：冻结
          provider-aware 动态块序列化、C1-0 类型复用、会话快照失效矩阵、tool/cache
          原子提交、C3 artifact 安全门、C4 same-turn mount 和缓存告警分类。研究入口
          与 C0/C1 规格已同步；日志：
          `docs/logs/2026-08-context-management-enhancement/v0.0.2-construction-decisions/`。
    - [x] **P0-1 / C1a**（2026-08-10）：stable prompt 改由 typed
          `PromptSection` 生产装配；Current Time/session、WM、Recall、Plan/Ask/Scheduled
          统一进入 history 后、current user 前的动态 envelope；provider 新增安全 transport
          capability，未知与现有生产 adapter 默认 `UserContextEnvelope`，Native 路径
          fail closed；reactive compaction 复用同一 turn section 快照。未改 tool 顺序、
          section cache、hash 观测或 `apply_cache_control`。日志：
          `docs/logs/2026-08-context-management-enhancement/v0.0.3-c1a-typed-stable-prefix/`。
    - [x] **P0-2 / C1c**（2026-08-10）：`ContextBuilder` 按 `session_key`
          缓存四个 SessionStable section 与渲染结果，维护单调 `prefix_version` 和强类型
          break reason；无事件时不重复读取 AGENTS/Skills/Frozen Core/L1。mask 与
          provider startup revision 分别只刷新 `MaskAndIdentity` 和
          `MemoryPolicyAndIndex`；reset/delete/shutdown 已接生命周期，compact 不清缓存。
          T5–T6 已覆盖；未引入 `prefix_hash`、观测或 `apply_cache_control`。日志：
          `docs/logs/2026-08-context-management-enhancement/v0.0.5-c1c-session-section-cache/`。
      - [ ] **C1c 外部 L1 apply 通知接线**：Manager 的 BML/Laputa 治理 apply 使用独立
            store/service 路径，当前不会递增 AgentLoop 持有 provider 的 startup revision。
            后续增加 Manager → runtime control → provider refresh/section invalidation 通知，
            并覆盖成功 apply、恢复 apply 与幂等重放；相关入口：
            `agent-diva-manager/src/handlers/laputa.rs`、
            `agent-diva-agent/src/runtime_control.rs`。
      - [ ] **C1c Skills reload 入口接线**：`invalidate_skills` 已提供显式失效 API，
            但 Manager skill upload/delete 与 `memory_distill` 尚未通知 AgentLoop；后续接入
            runtime reload 事件，保持“文件变化不轮询、显式事件生效”的会话语义。
    - [x] **P0-3 / C1b**（2026-08-10）：`ToolRegistry` 新增显式
          `ToolSchemaPartition::{Core, Deferred}`，definitions 固定为 CORE 字典序连续前缀 +
          MCP/custom DEFERRED 字典序后缀；schema JSON object 递归规范化键序且保留 array
          顺序。T4 已覆盖重复调用、反向注册及独立 `ToolAssembly` 重建的完整序列化字节
          一致性；现有 schema 均为静态值，未引入可选 session schema cache。未修改
          `apply_cache_control`。日志：
          `docs/logs/2026-08-context-management-enhancement/v0.0.4-c1b-tool-schema-stability/`。
    - [x] **P0-4 / C1d**（2026-08-11）：SHA-256 system/tools/per-tool hash、
          provider/model/policy/TTL 分桶与 cache read/create 趋势已接真实 provider call；
          expected/policy/undeclared/miss 分类按 DEC-CTX-G 落地，连续两次显著 miss 才告警。
    - [x] **P0-5 / C1d**（2026-08-11）：cache-control 仅锚定第一条 stable system
          与 CORE 工具段末；OpenAI-compatible 和原生 Anthropic wire 均覆盖，禁用 provider
          不注入。缓存 usage 透传 LLMResponse 与向后兼容 JSONL ledger。日志：
          `docs/logs/2026-08-context-management-enhancement/v0.0.6-c1d-cache-observability/`。
    - [x] 测试 T1–T8 落地
      - [x] **T1–T3**：clock / WM / Recall 变化不改变 stable prefix
      - [x] **T4**：同工具集 schema 完整序列化字节一致；CORE/DEFERRED 分区内字典序
      - [x] **T5–T6**：mask 仅刷新 mask section；同运行时 L1 revision 仅刷新 memory
            section，均递增一次 prefix version 并携带声明原因
      - [x] **T7–T8**：Anthropic cache read/create 透传 observer/ledger；多 system
            仅 stable 首块、混合工具仅 CORE 段末携带 cache-control
  - [x] **CTX-C2：分层 ContextBudgetPlan + AssemblyReport**（ADR-CTX-2）
        （2026-08-11）：新增 typed BudgetLayer/Fragment/Plan/Report，稳定前缀、
        CORE/DEFERRED schema、History、ToolResult、WM、Recall、当前回合均进入实际
        token 计量；`system_budget_ratio` 仅作稳定层上限，不再冒充已消费 token。
        Recall 在层级/总量压力下显式 Drop；主动 macro compact 与失败后的
        `legacy_count_cap` 均记录原因；provider call 输出无正文的分层报告。
        未提前实现 C3 artifact/ref 或 C4 deferred mount。日志：
        `docs/logs/2026-08-context-management-enhancement/v0.0.8-c2-layered-budget-report/`。
  - [x] **CTX-C3：工具结果引用化 + microcompact**（ADR-CTX-3；遵守 memory write-path 契约）
        （2026-08-11）：大于 12k 字符的完整脱敏工具结果写入 session-bound
        `.agent-diva/tool-artifacts/`，Prompt 仅保留版本化 opaque ref、3k preview、
        tool-call/status/size/SHA-256 与读取提示；`read_tool_result` 由 ToolAssembly
        绑定 workspace/session，范围读取最多 12k 字符。统一 registry/tools 80k
        fallback seam；C2 soft limit 触发时按最旧优先把 4k 以上旧成功结果
        microcompact，保留最新工具组、错误和 assistant/tool 配对，并在 cache observer
        前声明 expected deletion。重启恢复、TTL/容量/损坏/越权/session 删除均由
        artifact store 管理；不进入 BML/Laputa 或 Experience Journal 正文。日志：
        `docs/logs/2026-08-ctx-c3-tool-artifacts/v0.0.9-tool-result-artifacts/`。
  - [ ] **CTX-C4：CORE/DEFERRED 工具 + tool_search + Recall 测试矩阵**（ADR-CTX-4）
  - [ ] **CTX-C5：长任务压缩/恢复验收用例**（ADR-CTX-5）
  - [ ] **（分轨）Plan Mode 物理限制状态机** — 非 Context 主线
  - [x] **（分轨）Subagent Git Worktree 隔离** — 非 Context 主线
        **2026-08-11 用户决策：明确取消，不实施。**

- [ ] **GA-MEM-PARITY：GenericAgent 功能对齐 × Memory/Laputa/AutoDream 完全可用** `sev-P0`
  2026-08-05 完成只读盘点：相对 GenericAgent，Agent 侧记忆管理工具面基本缺失
  （无 add/list/search/update/remove/distill），prompt 仍承诺 “memory tools”；
  Laputa/Typed 的 `sync_turn` 多为 pending proposal 而非即时权威；工作记忆、L0–L4
  分层纪律、主动蒸馏未产品化；AutoDream 有底座与手动 API，自动触发与审查闭合待证。
  权威入口：
  [`docs/logs/2026-08-05-memory-ga-parity-inventory/v0.0.1-gap-inventory/inventory.md`](docs/logs/2026-08-05-memory-ga-parity-inventory/v0.0.1-gap-inventory/inventory.md)。
  实施按 inventory Wave 0–5；**禁止**回退 Mentle 或 “file_write 即权威”。
  验收见同目录 `acceptance.md`。
  **决策已冻结（2026-08-06，用户拍板，见 inventory §10.1）**：
  1) 混合分级写入（低风险即时 apply / 高风险审批）；2) L3 经验 Skills 为主
  （复用 `docs/architecture/skill-sop-unification.md` 决策）；3) 工作记忆 Session
  store（distill 显式晋升）；4) consolidation 降级为兜底；5) authority_mode
  统一默认 Typed（F10 随 Wave 0 修复）。
  **当前状态：Wave 0/1/2/3/4/5/6 已完成（2026-08-07）；延期项归 G2D+ / 独立 Wave。**
  - [x] **WAVE0-HONEST-CONTRACT：诚实与契约** `sev-P0`
    - [x] 修 prompt 假承诺（A8）：`deb5754b` context.rs 诚实降级文案 +
          负面回归测试（Wave 1 工具落地后恢复指引）
    - [x] 统一 `authority_mode` 默认与缺失配置语义为 Typed（F10，消分叉）：
          `b07a0818` schema.rs 两处 serde/enum default + 测试；行为变更
          （无 memory 段配置出箱走 Typed）已记录；AppState::new fixture 同步
    - [x] `sync_turn` 状态机诚实化（A9/H2）：`679d718d` 新增
          `ProposalCreated` 变体，Laputa proposal 路径不再谎报 `Persisted`；
          consolidation 视为成功；回归测试
    - [x] 写清 Working / Long-term / AutoDream 分工文档（G10/H5/C6）：
          `docs/architecture/memory-write-paths-contract.md`（含 tombstone
          过滤 G3、AutoDream 去重 G4、consolidation 兜底约束）
  - [x] **WAVE1-MEMORY-TOOLS-P0：Agent 记忆工具 CRUD（功能对齐核心）** `sev-P0`
    - [x] `memory` 工具面：add / list / search / update / remove + distill
          （A1–A7），注册进 ToolAssembly 与 mask 策略：`bd04cfc5`
          `agent-diva-tools/src/memory_*.rs` 六工具 + lib.rs 导出 +
          ToolAssembly `with_memory_provider` + builtin `memory` gate +
          `build_agent_tools` 接线
    - [x] 接到 MemoryProvider 扩展 API（不只 4 个 lifecycle hooks）：
          `1e40bf16` core `memory/crud.rs` 请求类型 + `MemoryCrudOutcome` +
          trait 6 个默认方法；`5a742c82` Typed 实现（put / list /
          search_visible / proposal 三路）
    - [x] 混合分级路由：低风险（用户明确要求记住的事实/偏好）即时 apply；
          高风险（删除/覆盖既有权威、敏感内容）走 proposal + 审批
          （S2：add→即时 put；update→MemoryPatch；remove→Deprecation；
          distill 新建即时/覆盖 SopCreate，W1-2/W1-3）
    - [x] Legacy 模式 proposal-first（不直写 MEMORY.md 冒充权威）：
          `522df4d0` LegacyCrudMemoryProvider（读走 MemoryManager，写全走
          Laputa proposal + `MemoryGovernanceCoordinator::open_lazy` submit）
    - [x] 写结果诚实语义（A9）：`applied` / `proposal_created`(含 id) / `failed`
          （`MemoryCrudOutcome` serde tag="status"；工具结果三态 JSON；
          context.rs 恢复工具指引 + 回归测试）
    - [x] 单元 + 集成测试覆盖成功与失败路径（core 682+/laputa 20+/
          agent 378+/tools 12+ 全绿；全 workspace `just test` 仅 CLI
          wiremock 502 既有失败 `CLI-WIREMOCK-502-PREEXISTING`）
    **Wave 1 决策已冻结（2026-08-06，inventory §10.2）**：W1-1 多独立工具
    （memory_add/list/search/update/remove/distill）；W1-2 按 action+对象状态
    分级（add 新事实=低风险即时，update/remove=高风险审批，不做内容检测）；
    W1-3 distill 新建即时、覆盖走审批；W1-4 结果返回不自动注入（热注入归
    Wave 3）。实施约束（tombstone 语义、FTS5/legacy 降级、distill→SKILL.md
    evidence）见 inventory §10.2。
    **闭环修订（inventory §10.3，2026-08-06）**：Wave 1 增加两项——
    (G1) distill 最小版输入=会话上下文（checkpoint evidence 在 Wave 2 扩展）；
    (G3) memory_remove 产生 tombstone（复用既有 apply 基建）；注入过滤基建
    已存在（typed_provider 启动渲染 + typed_store FTS 均过滤 tombstone），
    补「删除后不再出现」验收测试，遗漏路径再补。
  - [x] **WAVE2-LAYERS-WORKING-MEMORY：分层与工作记忆（inventory §6 +
        §10.3 G1 承接）** `sev-P0`
    - [x] L1 预算注入 + pointer（B2/B10）：启动注入改为有界 L1 索引——
          `render_l1_index_line`（≤80 字符预览）/`render_l1_index_block`
          （`2e70d92c`）+ `MemoryConfig.l1_index_lines`（默认 30，
          `7b30dc72`）；全文不再注入，取详情走 `memory_search`/`memory_list`
          按 id；Typed 与 Legacy（MemoryManager）同步；0 预算/空权威不注入
    - [x] Working checkpoint 工具与注入（C1–C3）：`update_working_checkpoint`
          工具（`c8111c22`，session key 由装配注入）→ session-scoped
          AppliedAuthority 记录（scope.session_id，启动渲染/列表天然排除）；
          每轮 `prepare_runtime_context` 注入 `## Working Memory` 块
          （`b1c101f9`，prefetch 索引随之移位）；key_info/related_sops 结构化
          渲染；on_session_end（含 loop 退出按 `SessionManager` 会话枚举）
          以 supersedes tombstone 清理（`2e70d92c`/`b1c101f9`）
    - [x] L0 policy 文本（B1/B8/B9 部分）：`L0_MEMORY_POLICY` 三条
          （Action-Verified / 禁止易变状态 / 最小指针）注入
          `## Memory Management Policy` 段（`b1c101f9`）；B9 完整 tool-result
          强制校验留 Wave 5（条目化跟进）
    - [x] G1 承接：distill evidence 扩展——`memory_distill` 可选 `evidence`
          参数（`c8111c22`）→ 新建写 `skills/<name>/EVIDENCE.md`、
          覆盖进 proposal excerpt、audit 标记（`2e70d92c`）
    - [x] U5 验收：长任务中途不丢关键上下文——checkpoint 每轮稳定注入，
          会话结束清理（测试：注入位置/空块跳过/清理后不可见）
    **Wave 2 决策基线（inventory §10.1 #3 + §10.3）**：工作记忆=Session
    store（易失、非权威，distill 显式晋升）；G1 承接 checkpoint evidence；
    会话异常退出残留清理与 B9 强制校验归 Wave 5（GC 条目）。
  - [x] **WAVE3-MEMORY-READ-CLOSURE：读侧闭环（prefetch 生产注入 + 启动一致）** `sev-P0`
    - [x] Typed prefetch 生产注入可用（D4 从 shadow 转生产，配置出箱即开）：
          `3ae7975d` typed_provider::wave3_tests::recall_returns_prompt_block_with_recalled_content
          断言 prompt_block 含召回关键字；turn/context.rs:390-425 注入路径已生产；
          authority_mode 默认 Typed（F10 Wave 0）出箱即开
    - [x] Legacy prefetch 不静默 Failed，有可理解降级（D2/D3）：
          `e66cfa9b` turn/context wave3_tests::legacy_prefetch_failure_leaves_messages_untouched
          断言 [system, user] 不增删；manager.rs:292-307 明确 reason 文案已可读
    - [x] 启动注入与 typed applied authority 一致（F5/H3 验收：apply 后
          FTS/startup 一致）：`3ae7975d` wave3_tests 覆盖
          memory_add_visible_in_next_startup_rendering /
          apply_then_search_remains_consistent_across_reopens；发现并修复
          supersedes-targeted 记录泄漏到启动渲染与 search 的真实 bug
          （新增 TypedMemoryStore::superseded_target_ids + 两处过滤接线）
    - [x] ~~**延期（Wave 5 或独立 slice）**~~：同会话热注入策略（F4，W1-4 延期项）
          **已在 Wave 6 S2 完成**（`56dd15d5`）——`startup_markdown` 改为
          `RwLock<Option<String>>`；CRUD 写入成功后调 `refresh_startup_markdown()`
          重新渲染缓存；同会话 `system_prompt_block` 即时可见新内容
    - [x] U1「记住→下次会话还在」、U2「你还记得吗」以本 Wave 为通过前提：
          自动化证据已具备（apply→FTS→startup 一致性测试）；真机联通 smoke
          归 G2D+ 桌面验收一并执行（规则 smoke-test-required-for-user-visible-change
          由 G2D+ 阶段覆盖）
  - [x] **WAVE4-AUTODREAM-G4：AutoDream 去重（双写边界收口）** `sev-P0`
    - [x] `LaputaService::applied_authority_digests`（`f042bba4`）：返回
          全部活跃 AppliedAuthority 长时记录的 content digest；四重过滤
          （AppliedAuthority / 无 tombstone / 无 session scope / 非
          supersedes 目标）；缺库（TypedMemoryStore 未创建）优雅返回空；
          新增 `LaputaError::InvalidState(String)` 变体 + `invalid_state`
          API code
    - [x] AutoDream Worker 双路 digest 接入（`a39638bb`）：
          `worker.rs reflect()` 将 laputa section digest（legacy 路径）与
          typed authority digest（新路径）合并（HashSet 并集）写入
          `BoundedReflectionInput.existing_memory_digests`；typed 读失败
          降级为 `tracing::warn!` + 空数组，不阻断 run
    - [x] CandidateGate Duplicate 端到端测试（`a39638bb` worker wave4_tests）：
          `candidate_duplicate_against_typed_authority_is_rejected`、
          `candidate_fresh_against_typed_authority_is_accepted`、
          `superseded_authority_record_no_longer_blocks_duplicate_candidate`
    - [ ] **Wave 4 延期项（归 G2D+ / 后续独立 Wave）**：G1 手动触发端到端
          验收、G2 自动阈值触发联通、G3 多源输入闭环、G5 候选→proposal
          端到端、G6 审查 UI、G7 节律报告可见、G10 与 agent 即时记忆分工
          真机验证、G11 L4/salient 等价、G12 Action-Verified 公理对齐——
          均为产品/真机验收，本 Wave 最小闭环不吞下
  - [x] **WAVE5-CONSOLIDATION-AND-GC：巩固与清理（收口）** `sev-P0`
    - [x] S1 Working memory GC（A）：`865f527d` `TypedMemoryStore::gc_session_scoped`
          + `gc_stale_session_scoped`（物理 DELETE session-scoped 记录）；
          `TypedLaputaMemoryProvider::on_session_end` 调用 gc_session_scoped；
          `open()` 末尾调用 gc_stale_session_scoped；失败仅 warn 不阻断；
          三个 wave5_tests 覆盖目标 session 隔离/孤儿清理/空库
    - [x] S2 Superseded gate 变体（B）：`16aa46ed`
          `CandidateRejectionCode::Superseded`（优先级 > Duplicate/Suppressed）；
          `BoundedReflectionInput.superseded_memory_digests`（`#[serde(default)]`
          向后兼容）；`LaputaService::superseded_authority_digests`（缺库优雅空）；
          worker.rs 双路 digest 扩展；3 个 service-level + 3 个 gate-level 测试
    - [x] S3 F6/F7 端到端验收（C）：`043beb5b`
          `tests/wave5_acceptance.rs` 四个集成测试：
          `f6_rollback_clears_fts_and_startup`（governed apply → rollback →
          FTS + startup 不含目标）、`f6_rollback_is_idempotent`（幂等返回
          true/false/false）、`f7_tombstone_lifecycle_filters_startup_and_search`
          （tombstone → 下次 startup + search 过滤；已知 memory_list 不过滤
          superseded 记录——记录为延期项）、`f7_tombstone_target_missing_stores_tombstone_record`
    - [x] S4 Consolidation 条目化兜底（D）：`3da2bb7b`
          `save_memory` prompt v3 要求 `[{action,id?,content}]` 结构化 JSON；
          逐条 dispatch 到 `memory_add/update/remove`（`MemoryCrudOutcome` 统计
          applied/proposed/failed）；非数组降级到 `sync_turn` proposal；
          `agent_diva_tools::distill_guard` 跨 crate flag（`memory_distill` 成功后
          置位 → `should_consolidate` 跳过）；3 个 wave5_tests + prompt 契约测试
    - [ ] **Wave 5 延期项（归 G2D+ / 后续独立 Wave）**：
      - B9 完整 tool-result 强制校验（Wave 2 延期项；**Wave 6 S3 已实现软
        advisory 子集** `c2e17e5e`——完整强制归后续独立 Wave）
      - ~~F4 同会话热注入~~（**已在 Wave 6 S2 完成** `56dd15d5`）
      - F7 GUI 可见 tombstone 历史（S3 已证自动化路径；GUI 联通归 G2D+）
      - ~~`memory_list` 不过滤 superseded 记录~~（**已在 Wave 6 S1 修复**
        `2572a478`——`superseded_target_ids()` 已接入 memory_list 过滤）
      - G1/G2/G3/G5/G6/G7/G10/G11/G12 真机端到端验收（Wave 4 延期项）
  - [x] **WAVE6-PRE-G2D：真机前收口（代码级缺口修复）** `sev-P0`
    - [x] S1 memory_list superseded 过滤（A）：`2572a478` `memory_list` 增加
          `superseded_target_ids()` 过滤（与 memory_search/startup 同模式）；
          wave5_acceptance 测试断言修正为 NOT-in-list
    - [x] S2 F4 同会话热注入（B）：`56dd15d5` `startup_markdown` 改为
          `std::sync::RwLock<Option<String>>`；提取 `render_startup_index` 辅助
          函数；新增 `refresh_startup_markdown()`；CRUD 写入成功后刷新缓存；
          `f4_memory_add_visible_in_same_session_startup` 测试验证
    - [x] S3 B9 soft evidence advisory（C）：`c2e17e5e`
          `MemoryCrudOutcome::Applied` 新增 `evidence_advisory: Option<String>`
          （None = 有 evidence；Some = advisory 文案）；`MemoryAddRequest` 新增
          `evidence_refs: Vec<EvidenceRef>`（serde default 向后兼容）；
          `memory_add` 传 evidence_refs 到 MemoryRecord 并在空时设 advisory；
          全 workspace Applied 构造/析构更新；wave6_tests 两个用例
    - [x] S4 docs 收口 + TODOLIST（D）：本提交
    - [ ] **Wave 6 延期项（归 G2D+ / 后续独立 Wave）**：
      - B9 完整 tool-result 强制校验（S3 仅做软 advisory；完整强制需改
        MemoryAddRequest schema + agent_loop 证据链跟踪）
      - [x] **BML 记忆管理可视化（P0 真机验收发现）**：内置工具无法完成记忆
        删除（`memory_remove` 走 proposal 但无审批入口）；用户看不到 agent
        当前记忆内容；人格与记忆需要 BML（Basic Memory Layer）用户可视化
        管理界面。**已在 2026-08-09 Garden 记忆工作台批次完成**（GARDEN-MEMORY-UI）：
        侧栏拆「人格 / 记忆」双版块（按真实三层架构）；manager 新增
        `/api/bml/memories` 只读端点 + remove proposal 端点；GUI 新增记忆仓库
        （搜索/过滤/详情/删除→审批中心跳转）。见
        `docs/logs/2026-08-garden-memory-ui/`。
      - **Laputa 设计同步** → LAPUTA-COGNITIVE-SYNC 已完成（S0–S7，
        见上方 Active Plan 区与 docs/logs/2026-08-laputa-cognitive-sync/）。
      - **记忆检索增强（待决策）**：当前 `memory_search` 基于 FTS5 全文
        匹配，召回质量与语义检索、上下文相关性排序等增强方向待决策。
        排期待决策。
      - G1/G2/G3/G5/G6/G7/G10/G11/G12 真机端到端验收（Wave 4 延期项）

- [x] **E0–E7：AutoDream–Laputa 开箱可用纵向闭环** `sev-P0`
  按 Experience Journal、可恢复 Orchestrator、受限 Reflection、Candidate Gate、
  Laputa proposal/统一治理、typed apply、Recall feedback、一体化 GUI、恢复与发布门
  逐切片完成。普通 AutoDream trigger 必须启动真实 worker；禁止固定模板候选、
  直接写 Memory、自批准或静默降级。每切片四件套、独立提交、不 push。
  相关：`docs/dev/autodream-laputa-product-closure/09-project-management.md`。
  - [x] **E0：运行时表征与诚实产品状态**：普通手动触发会执行受限 worker 并返回
    终态；失败具有稳定的无载荷原因码；Evolution 页面明确提示当前仍是规则式候选、
    尚未形成可用闭环。
  - [x] **E1：Experience Journal**：把真实任务执行结果、工具结果与错误转成有界、
    可追溯、可脱敏的反思输入。
    - [x] **E1A 在线执行证据**：AgentLoop 唯一工具 seam 记录 payload-free 结果，
      具备确定性 ID、幂等、workspace 隔离、物理 retention 和损坏行拒绝；AutoDream
      优先收集该证据。
    - [x] **E1B 离线回填与回滚**：从已有 session tool-result 生成 dry-run manifest，
      显式 apply，按 manifest 回滚；不得读取或复制完整工具输出。
  - [x] **E2 可恢复 AutoDream 编排器**：手动与报告触发统一持久化
    `queued → gathering → reflecting → validating → publishing → completed` 阶段、
    attempt 和 deadline；Manager 异步派发并在启动时恢复中断任务；发布重放复用
    确定性提案，旧版不完整运行 fail closed。
  - [x] **E3 Reflection/Candidate Gate**：生产 Reflection 复用现有 provider resolver，
    使用有界、PII 脱敏输入与无工具 JSON schema；候选包含真实 proposal type、scope、
    sensitivity、confidence、value 与失效条件，并在发布前拒绝无证据、仅 compaction、
    forged evidence、重复、直接矛盾、低价值、越 workspace/容量、prompt injection
    和敏感内容。无 provider 明确失败，无合格候选诚实完成为 `no_candidates`。
  - [x] **E4 提案治理与 rejection suppression**：AutoDream 候选一对一进入
    PendingReview；Memory 决策继续使用 core Governance Ledger，编辑后旧 request/receipt
    失效；决策已提交但 proposal 状态未落盘时可重试补齐；拒绝内容以 90 天、1000 条、
    payload-free digest suppression 抑制原样重提，显著变化可重新进入 gate。
  - [x] **E5 typed apply、Recall 与反馈**：批准后的 AutoDream 提案映射为 canonical
    `MemoryRecord` 并保留 run/evidence/governance provenance；AgentLoop 在唯一 turn
    终态 seam 提交 payload-free Recall 成功/失败/纠错反馈；下一次 Reflection 可把
    纠错转为 governed deprecation，apply 时生成 content-free tombstone/supersedes，
    无效或缺失 target fail closed。
  - [x] **E6 一体化 Evolution Workspace**：单一页面汇总 typed authority/revision、
    AutoDream phase/attempt/输入覆盖、候选提案、治理动作、changelog/rollback 与
    payload-free Recall feedback；支持立即反思、取消、查看 run 提案、可视化编辑后
    生成新 proposal revision、批准并应用、拒绝、暂缓与回滚；typed degraded 原因
    显式展示。
  - [x] **E7 恢复与发布门**：完成 canonical workspace identity、单一副作用 seam、
    observability、恢复演练、全测试与发布候选自动化 E2E。
    E7 必须在关闭占用 debug 二进制的桌面进程后复跑 `just test`，并解决或规避当前
    Windows GUI lib test 的 MSVC `LNK1140` PDB 限制，不能以 `cargo check` 代替。

- [x] **Evolution 当前状态必须明确为不可用/建设中** `sev-P0`
  在 E0 characterization 前不得将现有页面标记为可用。允许且要求实施本闭环；
  不在闭环内的旧 AutoDream/Evolution 承诺继续冻结，完全无后端的入口必须隐藏、
  删除或明确 degraded。

- [x] **LAPUTA-COGNITIVE-SYNC：新 Laputa 认知分区回迁（S0–S7 全部完成）** `sev-P0`
  调研：桌面 Garden ADR-0002/0004/0008 认知分区。权威提案：
  [`docs/research/laputa-garden-cognitive-sync-2026-08/gap-and-migration-proposal.md`](docs/research/laputa-garden-cognitive-sync-2026-08/gap-and-migration-proposal.md)。
  **用户关键决策（2026-08-07，全部冻结）**：
  Q1 认知文件 workspace 级（`.laputa/cognitive/MEMRULES.MD`+`WORLD.MD`）；
  Q2 `LaputaSectionName` 5 个废弃变体一步硬删（落盘历史反序列化走
  unknown-variant 稳定失败）；Q3 **删除 SOUL.md/IDENTITY.md/USER.md/
  BOOTSTRAP.md/memory/MEMORY.md/HISTORY.md 整个人格文件层，人格与记忆
  完全 Laputa 治理**（内容先走治理审批迁移，文件入 `.laputa/legacy/`
  惰性存档；AGENTS.md/Skills 保留）；Q4 WORLD.MD AutoDream 首切片即可
  治理写入+人类可直编（confirmed+user claim 保护不变）；Q5=b 报告
  系统完全重构（边界四句：报告≠记忆、永不注入、agent 可选读自决、
  生成权威=report_system；删 Daily/Weekly/MonthlyPatch proposal 与
  记忆 kind 映射；节律自动触发补真机验证）；D1 **Mask 保留且与
  Laputa 互不影响**（面具=临时外在覆层，不改人格本体，不得解除
  commitment 红线，不进治理）；D2 报告产物迁 `.laputa/reports/`
  （只搬不改可回滚）。**不回迁 Mentle，不引入 garden Go 代码**。
  D3 **BML/Laputa/Garden 三层模型（2026-08-08 冻结）**：BML（Basic
  Memory Layer）= 记忆存储层（Mentle 继承者，实体 `.laputa/memory.sqlite3`
  + FTS5，唯一权威）；Laputa = 人格治理层（proposal/governed apply/
  audit/Frozen Core）；Garden = facade（用户可见记忆管理面，桌面 GUI，
  未实现）。AGENTS.md 定义已同步修正。代码现状：存储实体在
  laputa crate 内部未独立成层，抽层与否待后续决策。
  - [x] S8 BML 抽层调研完成（2026-08-08）：耦合盘点（H1–H6 验证）、
    Garden 三 module 同构对照、方案 A/B/C/D 对比与决策建议见
    [`docs/research/bml-layer-extraction-2026-08/bml-layer-extraction-research.md`](docs/research/bml-layer-extraction-2026-08/bml-layer-extraction-research.md)
  - [x] **决策已拍板（2026-08-08）：BML 抽层选 D——先 B（crate 内模块
    边界 + 防回归门）固化边界，远期视 Garden facade 落地升 A（全抽）**。
    依据调研报告 §8/§9。术语注：Garden 侧 Mentle = BML 的 Garden 侧
    对应物，非 mempalace-mentle。实施 B 与未来升 A 排期待决策。
  - [x] S9 **实施 B 完成（2026-08-08）**：新增 `agent-diva-laputa::bml`
    命名空间（存储核心 + 记录适配层公开面，`src/bml/mod.rs`）+ 治理层
    禁止直写 BML 写接口的负向守卫 `tests/bml_boundary_guard.rs`（挂
    `just bml-boundary-check`，纳入 `ci` 与 `e7-automated-release-gate`）；
    AGENTS.md 追加分层声明。提交 `9758ac32`。
  - [ ] 若未来升 A：GMH-23A 所有权条款修订 + §8 禁令 amendment（调研报告
    §7 已给论证与修订路径），时机 = Garden facade 落地。
  执行顺序：
  - [x] S1 cognitive/ 目录与 MEMRULES.MD（R1–R7，人类 only，永不注入）：`41e60a7e`
  - [x] S2 WORLD.MD（claim schema + AutoDream 治理写 + scope/budget 投影）：
        `5788eddf` + `b22b50e8`
  - [x] S3 Frozen Core 会话冻结语义（01–04 启动快照，写入次会话生效）：`1c97d7be`
  - [x] S4 人格文件层退役与内容治理迁移（Mask 保留为覆层）：`49e778e1`
        （persona-retire 迁移工具）+ `57044ba8`（BOOTSTRAP 装配除名）+
        `989a18a2`（soul 机制删除）
  - [x] S5 注册表 14→9 硬删收敛（5 废弃变体 + HistoryPatch/JournalNote，
        落盘历史容错跳过 + 稳定失败码）：`e61630b8`
  - [x] S6 报告系统边界重构（写权威收口+注入禁令+节律链路验证+产物迁 `.laputa/reports/`）：
        `23ea4b1e`（删 rhythm patch proposal 目标与记忆映射）+
        `856335b2`（禁 Rhythm Signals 入 prompt）+ `6def944e`（D2 产物迁移）
  - [x] S7 Context Plane 负向不变量矩阵（8 行矩阵移植为负向回归测试）：`2457239b`
  分支纪律：从 pro 切 `feat/laputa-cognitive-sync`；逐切片四件套 +
  单一 concern 提交 + `just ci`，不 push。
  收尾：四件套见 [`docs/logs/2026-08-laputa-cognitive-sync/`](docs/logs/2026-08-laputa-cognitive-sync/)；
  S6-4 节律真机验证以进程级集成测试覆盖，完整 daemon-cron 真机
  待 G2D+ 桌面验收补跑。

- [ ] **G2D+：全流程完成后的真实桌面最终验收** `sev-P0`
  自动化纵向 E2E 与发布门通过后再由用户执行。保留原批准、拒绝、编辑后批准、
  重复点击、重启恢复、回滚六场景，并新增“真实任务 evidence → AutoDream →
  proposal → apply → 新会话 Recall → rollback 后消失”。不得以自动化冒充真机。
  保留脱敏 ID、revision、桌面版本、界面结果和 Manager/Tauri 日志。

完成索引（已归档）：Ask 只读边界、GMH-24A/B/C、G2D migration revision 修复等见
[`docs/archive/todolist/completed-through-2026-07-30.md`](docs/archive/todolist/completed-through-2026-07-30.md)。

---

## Operational Rules

standing policy（非功能债，执行相关验证时遵守）：

- [ ] **真机里程碑须先提醒用户** 需要真实设备/桌面/外部集成时，先列出环境、步骤、
  观察点与失败诊断物，获配合后再验收；禁止用模拟冒充真机。
- [ ] **真实 API 使用桌面 `keys.txt`** 不得把密钥或未脱敏内容写入仓库、日志、
  夹具或提交。

---

## 人工测试验收汇总（Human / Real-Device Smoke）

> 所有需要**真实设备 / 真实 LLM / 人工 GUI 操作**的待验收项集中于此，作为统一
> 检查清单；原始条目仍保留在各自位置（勾选时两处同步）。执行纪律遵守上方
> Operational Rules：真机里程碑须先提醒用户，列出环境、步骤、观察点与失败
> 诊断物；禁止用模拟冒充真机。

### A. G2D+ 真实桌面最终验收（`sev-P0`，伞项；待自动化发布门通过后执行）

- [ ] 原六场景：批准、拒绝、编辑后批准、重复点击、重启恢复、回滚
- [ ] 新旅程：真实任务 evidence → AutoDream → proposal → apply → 新会话
      Recall → rollback 后消失
- [ ] F3：agent 产生 proposal → GUI/CLI 审批中心批准 → apply → typed 更新联通
      （原条目：GA-MEM-PARITY Wave 3 延期项 F3）
- [ ] F7：tombstone 历史在 GUI 可见（用户审批 → 历史面板）
- [ ] U1/U2：「记住→下次会话还在」「你还记得吗」真机联通
- [ ] S6-4：节律 daemon-cron 真机挂跑补跑（LAPUTA-COGNITIVE-SYNC 收尾项）
- [ ] GUI provider 重试/错误可见性人工 smoke（GUI-PROVIDER-RETRY-VISIBILITY /
      GUI-PROVIDER-ERROR-SILENT 修复后遗留）
- [ ] Wave 4 延期真机 E2E：G1 手动触发、G2 自动阈值、G3 多源输入、G5 候选→
      proposal、G6 审查 UI、G7 节律报告可见、G10 即时记忆分工、G11 L4/salient、
      G12 Action-Verified 对齐
- [ ] 产物要求：保留脱敏 ID、revision、桌面版本、界面结果和 Manager/Tauri 日志

### B. 独立人工 smoke（可在 G2D+ 之前单独执行）

- [ ] **M3 审批 HITL 集中人工 smoke**：候选 `2fbb07d3`；首道硬门禁 =
      WINDOWS-RELEASE-EXEC-ACCESS（release EXE OS error 5，需人工/策略授权）
- [ ] **SANDBOX-SAVE-FIX-DESKTOP-SMOKE**：GUI 沙箱设置保存——切换模式保存、
      检查 `~/.agent-diva/config.json`、清空 timeout 边界；步骤见
      `docs/logs/2026-08-sandbox-settings-save-fix/v0.1.0-sandbox-save-fix/acceptance.md`
- [ ] **CLARIFY-HITL 人工 smoke**：真实 LLM 触发 `ask_user` 的 CLI/GUI 验收；
      步骤见 `docs/logs/2026-08-ask-user-hitl-research/v0.2.0-ask-user-surface/acceptance.md`
- [ ] **gateway 端口配置验收**（`fix/small-fixes-batch`，待合并）：配置
      `gateway.port` 后 `gateway` 命令监听配置端口，未配置仍 3000；步骤见
      `docs/logs/2026-08-small-fixes-batch/v0.5.1-small-fixes-batch/acceptance.md`

### C. 与里程碑绑定的真机项

- [ ] **GMH-52 全量验收**中的真实 smoke 部分（排期决策：稍后；与 G2D+ 旅程重合）
- [ ] **G2D+ 前置**：Windows release 可执行启动恢复（WINDOWS-RELEASE-EXEC-ACCESS，
      `sev-P1`）——自动化无法推进，需系统策略/人工授权

---

## E2E 自动化验收汇总（Automated E2E）

> 以**自动化测试**形式执行的集成/端到端验收项（部分需要真实 key 或真实外部
> endpoint——仅执行环境需人工配合，测试本体是自动化的，不属于上方人工区块）。
> 原始条目仍保留在各自位置（勾选时两处同步）。

### 外部 endpoint（需真实 key，走桌面 `keys.txt`，勿入库）

- [ ] **StepFun 真实 endpoint E2E**：model pass-through 透传真机验证
      （单测已覆盖透传，缺真实 key 的 E2E；原条目：Reliability / Test Debt）

### 生产路径 E2E 证明（Wave 3 residual，原条目：Deferred Product）

- [ ] `enqueue_background_task` 生产路径 E2E 证明
      （`agent-diva-tools/src/enqueue_background_task.rs`、assembly / agent_loop）
- [ ] supervised subagent worker bootstrap 刻画（启动/排空/取消/重启生产路径测试）
- [ ] subagent 终态生命周期 E2E
- [ ] background task 上下文与预算继承 E2E

---

## Open Backlog

### feat/laputa-cognitive-sync 延期项

- [ ] **WORLD-MEMRULES-GATE：WorldGovernance submit 阶段 MemRules R6 拦截** `sev-P2`
  C6 已完成 Memory 侧 MemRules 注入；WORLD 侧需在 `WorldGovernance::submit` 中
  调用 `MemRules::load_or_default()`，违反 R6 时返回
  `WorldGovernanceError::Protected { reason: "R6: WORLD entry gate" }`。
  依赖 C7 接线完成后再做。
- [x] **FROZEN-CORE-RENDER-IN-PROMPT：Frozen Core 投影实际注入 system prompt** `sev-P2`
  ✅ 2026-08-08 验收：`agent_loop.rs` 两条 e2e（`first_run_onboarding_injected_when_frozen_core_empty` /
  `first_run_onboarding_absent_when_frozen_core_has_content`）证明 prompt 装配
  真实包含 Frozen Core 投影（"Frozen Core" 前缀）与空态引导块（"First-Run
  Onboarding"），并证明 populate 后引导块消失。
- [ ] **MEMRULES-DEFAULT-SEED-ALIGNMENT：默认 R1-R7 与生产治理策略对齐** `sev-P3`
  `DEFAULT_MEM_RULES_TEXT` 里的 R1-R7 来自认知设计，需要 PM/Winston 复核
  是否与现有 `PolicyRestriction` 语义、autonomy level 映射完全一致；必要时
  调整规则措辞或新增 R8。

### GA-MEM-PARITY Wave 3 延期项

- [ ] **F3：GUI/CLI 审批 memory 域端到端验收** `sev-P1`
  Wave 3 自动化证据覆盖了 `apply→FTS→startup` 一致性，但"agent 产生 proposal →
  用户在 GUI/CLI 审批中心批准 → apply → typed 更新"的联通验收归 GMH-52 /
  G2D+ 桌面验收一并执行（与 U7 旅程重合；依赖真机 GUI/Manager）。
  **排期决策（2026-08-10）**：稍后处理，本轮不启动。
- [ ] **F4：同会话热注入（apply 后同会话立即可见）** `sev-P2`
  Wave 3 只证"下次会话 prefetch/startup 可见"。apply 后同会话刷新
  startup_markdown 缓存或触发 re-prefetch 的机制未实现（typed_provider.rs:87
  `startup_markdown` 在 `open()` 一次性渲染并缓存；刷新需要重新 list+render
  或显式 invalidate 接口）。归 Wave 5 或独立 slice。
- [x] **F6：Rollback 端到端验收（changelog→FTS 清退→startup 不再出现）** `sev-P1`
  Wave 5 S3（`043beb5b`）`tests/wave5_acceptance.rs` 四个测试覆盖 governed
  apply → rollback → FTS + startup 清退 + 幂等性。
- [ ] **F7：tombstone 完整生命周期验收（U3 用户路径）** `sev-P2`
  Wave 5 S3（`043beb5b`）已证自动化路径：tombstone → startup + search 过滤。
  GUI 可见 tombstone 历史（用户审批 → GUI 历史面板）归 G2D+ 桌面验收。

### Product / Architecture

- [ ] **agent-diva-files clippy clean（预存在）** `sev-P3`
  `agent-diva-files/src/s3.rs:247` `empty_line_after_doc_comments` 警告
  （rust 1.94 clippy `-D warnings` 可复现）。Wave 2 与 Wave 3 baseline 均
  存在，stash 验证与本 Wave 改动无关。归后续清理 Wave 或独立 slice。

- [ ] **RG-CODE-GOV 后续分期（可选实现入口）** `sev-P2`
  原位治理设计完成；**禁止**回迁 deep-governance 大爆炸。
  已完成并归档：G0 characterization、G1 AgentLoop 瘦身。
  未做：G2 Manager handler 变薄、G3–G5 GUI Host/state/DTO。
  设计：`docs/dev/agent-loop-manager-gui-governance/`。
  与 GMH-40 副作用 seam 有交集时优先走 GMH 故事，避免双轨。

- [ ] **待决策：全仓库代码清理提案集合审批与实施决策** `sev-P3`
  提案集合：[`docs/logs/2026-08-05-code-audit-proposal/v0.1.0-code-audit-proposal/`](docs/logs/2026-08-05-code-audit-proposal/v0.1.0-code-audit-proposal/)。
  经 4 路子代理全量深度审计完成：需用户决策是否批准并排期清理（包含 Migration 1389 行未挂载代码、Memory 影子比对逻辑、16 个未用 Tauri Command、10 个废弃 HTTP 路由、Vue 双通道 SSE 与通道死逻辑等 6-Wave 清理案）。


### GMH 未完成（Phase 3–5）

> Phase 0–2（GMH-00..24）已完成，详见 archive。以下为仍开放 story。

- [x] **GMH-30：统一审批协调器** `sev-P1`
  Plan / Sandbox / Memory 经同一协调接口；执行器只消费有效 receipt。
  suspend/resume、重启恢复、取消、超时、重复响应、多客户端。
  - [x] **GMH-30A：领域协调器与状态机**：core 协调器统一组合纯策略与 append-only
    ledger；Plan/Sandbox/Memory envelope 共用 Pending 入口，安全允许和策略拒绝不制造
    审批记录，decision/state 保持 version CAS、幂等和 payload-free 约束。
  - [x] **GMH-30B：receipt 消费与恢复控制**：执行器只消费有效 receipt，并补齐
    - [x] **GMH-30B1：Sandbox durable receipt 与重启撤销**：命令审批使用统一
      `.laputa/governance.db`，Once 在执行前消费，session 五分钟到期，global rule
      仅在 Rule receipt 后持久化；启动分页撤销当前 workspace 的 Pending/Allowed，
      不保存或重放命令正文。
    - [x] **GMH-30B2：Plan/Memory receipt 最终统一**：收口 Plan 审批表与 Memory
      apply 的执行消费和恢复语义。
    suspend/resume、重启恢复、取消、超时、重复响应和多客户端协调。
- [x] **GMH-31：Manager API / SSE / Tauri 契约** `sev-P1`
  pending/详情/approve/edit/reject/cancel/审计；幂等键与版本前置条件；
  typed reason codes 与事件序列。统一 service 投影三域 authority，HTTP 提供稳定
  list/detail/decision/cancel 与 durable cursor SSE；Tauri 仅负责 transport，Rust fixture
  由 TypeScript guard 共用验证，旧 Command/Plan/Laputa wire contract 保留。
- [x] **GMH-32：GUI 决策中心与就地审批** `sev-P1`
  全局 pending badge/抽屉与 Chat 就地卡统一消费 Manager projection；支持三域筛选、
  风险/证据/diff/TTL、Command grant、Memory 源页面编辑/应用导航、事件去重、stale
  刷新和 outcome-unknown 禁止重发，并以文字状态、焦点圈与 44px 控件满足键盘边界。
- [x] **GMH-33：CLI/headless 行为** `sev-P2`
  `approvals review/list/decide/cancel` 统一消费 Manager projection，批量交互必须显式
  选择；`agent` 默认 fail-closed，只有 `--approval-mode queue` 且 Manager 可用时才返回
  可查询的 Plan/Memory Pending，Command queue 因原文不持久化而撤销并返回稳定失败码。
  shell / Memory 高风险 / Plan、JSON/exit code 与 unavailable 路径均有自动化覆盖。
- [x] **GMH-40：Agent Loop 单一副作用 seam** `sev-P1`
  组装/pre-call/执行同一治理快照；turn 分段可取消可度量；子代理/cron 禁止提权。
- [ ] **GMH-41：自治预算与熔断** `sev-P2`
  turn/session/day 限额；拒绝风暴熔断；离线高风险排队或拒绝。
- [x] **GMH-42：治理可观测性与审计** `sev-P2`
  decision latency、人工等待、deny/stale receipt、Memory apply/rollback 指标与
  correlation 证据链。
- [ ] **GMH-50：兼容迁移与 feature flags** `sev-P2`
  不得重新引入 Mentle；禁止长期双写。
- [x] **GMH-51：安全与数据恢复演练** `sev-P1`
- [ ] **GMH-52：全量验收** `sev-P1`
  `just fmt-check` / `check` / `test`、deletion-proof、GUI、真实 smoke。
  **排期决策（2026-08-10）**：稍后处理，本轮不启动。
- [ ] **GMH-53：灰度与清理** `sev-P2`

里程碑（更新后）：

- [x] M0 / M1 / M2 — 见 archive（含 GMH-24 clean-break）
- [ ] **M3** 审批 HITL 闭环（GMH-30..33）— **仅** Plan/Sandbox/Memory 治理审批，
  **不含** Agent 主动 `ask_user`/clarify（见下方 `CLARIFY-HITL`）
  - [x] **M3-GOAL-PREP：长任务执行资料包**：冻结 Goal 边界、当前缺口、
    Plan/Memory/Command 消费与恢复矩阵、统一 API/SSE/Tauri、全局抽屉与 headless
    行为、自动化/人工验收和四个阶段停点。权威入口：
    `docs/dev/governance-m3-goal/README.md`。GMH-30/31/32/33、纳入的代码债和最终
    自动化矩阵均已完成；候选 `2fbb07d3` 等待一次集中人工 smoke。Windows release
    EXE 在当前自动化会话仍被 OS error 5 拒绝，作为人工矩阵首个硬门禁保留。
- [ ] **M4** Agent Loop 接入（GMH-40..42）
- [ ] **M5** 灰度发布（GMH-50..53）

统一 DoD：设计/威胁模型；成功/拒绝/超时/并发/重启测试；用户可见路径 smoke；
`docs/logs` 四件套；单 concern Conventional Commit；不擅自 push。

### Reliability / Test Debt

- [ ] **WORKSPACE-GUI-TOOLING-LOAD-FLAKES: full-suite startup/timeout tests are load-sensitive** `sev-P2`
  2026-08-11 CTX-C2 validation observed two non-deterministic full-suite failures:
  GUI `embedded_server::tests::embedded_gateway_serves_health_endpoint` failed to bootstrap the
  embedded manager once, then passed focused; a second `just test` failed in
  `agent-diva-tooling --lib`, whose complete 32-test focused rerun passed. A subsequent full
  `just ci` passed. Isolate startup resources and timeout-sensitive registry tests so workspace
  load cannot cause transient failures; CTX-C2 does not modify either subsystem.

- [x] **GUI-PROVIDER-RETRY-VISIBILITY: 前端展示 provider 重试/未响应状态** `sev-P2`
  2026-08-06 用户反馈（与 `GUI-PROVIDER-ERROR-SILENT` 同场景）：期望前端在重试期间
  显示「未响应，重试中 (1/4)」类状态，而不是静默等待约 128s 后突然失败。
  现状勘察：`send_with_retry`（`agent-diva-providers/src/retry.rs:76-130`）只有
  debug/warn 日志，**无任何事件/回调通道**（attempt/max/delay 对外不可见）；
  `AgentEvent` 无 Retry 变体；chat SSE（`handlers.rs`）、Tauri 桥（`commands.rs`）、
  前端（App.vue/ChatView）均无重试状态概念。
  修复方向：provider 层暴露重试回调或事件（attempt/max_retries/model/delay_ms）→
  `AgentEvent` 新变体（如 `ProviderRetry`）→ chat SSE 事件映射 → Tauri 桥转发 →
  App.vue 监听 + ChatView 在运行中消息上渲染「未响应，重试 (n/m)」。
  注意：`chat`/`chat_stream` 是 provider trait 签名，加回调的改动面较大，需评估
  兼容性（可选注入 callback 或独立事件 sink）。
  **已修复（2026-08-06）**：`cf567087`/`78b813c0`/`51c6c949`/`97a2195e`/
  `5ad64b5e`/`d4050d38`（详见 `docs/logs/2026-08-gui-provider-error-visibility/
  v0.1.0-provider-error-retry-visibility/`）。实现采用「trait 默认方法
  `set_retry_listener` + 每次调用前设置/调用后清除」方案，`ProviderTap` 转发，
  并发安全（设置与 `chat_stream` 调用间无 await）。人工 smoke 待实机执行。

- [x] **GUI-PROVIDER-ERROR-SILENT: GUI 对 provider 请求失败（重试耗尽）无提示** `sev-P2`
  2026-08-06 观测：DeepSeek 连接失败（`unexpected EOF during handshake`）重试
  4 次、耗时约 128s 后失败，GUI 全程无任何错误提示（用户只能从日志发现）。
  日志证据：`retry.rs:96` attempt 1-3 → `audit.rs:245` ProviderCallCompleted
  status=error → `agent_loop.rs:824` handle_inbound Failed。
  **根因线索（已勘察，未修）**：错误事件链路存在——`handle_inbound` 失败 →
  `emit_error_event`（`loop_runtime_control.rs:154-169`）→ bus →
  manager `handle_chat`（`runtime_control.rs:35-53`）→ chat SSE `error` 事件 →
  Tauri 桥 emit `agent-error`（`commands.rs:1312-1320` 带 request_id；
  `commands.rs:2531-2532` background 流为裸字符串）→ App.vue `agent-error`
  监听（`App.vue:2337`）。**疑似断点在 App.vue:2349 的 request_id 匹配过滤**
  （`requestId !== activeStreamRequestId` 即 return，长重试期间前端流状态
  可能已超时/清空导致不匹配被丢弃），或主聊天流桥接在长时间重试后未送达。
  修复方向：排查 activeStreamRequestId 生命周期与桥 stream_request_id 一致性；
  保证最终失败无论 request_id 是否匹配都渲染错误气泡。
  **已修复（2026-08-06）**：真正根因是 manager `handle_chat` 60s 无事件超时
  静默断开 SSE（重试 128s 期间错误事件丢失）+ Tauri 桥断流时静默 `Ok(())`
  （前端永久挂起）。修复：`97a2195e`（`forward_chat_events` 双段超时：60s 发
  `ProviderStalled` 非终止提示、再 60s 发明确断开错误）、`5ad64b5e`（Tauri 桥
  `saw_terminal` 断流兜底 emit `agent-error`）。App.vue:2349 request_id 过滤为
  合理防护，未改动。人工 smoke 待实机执行。

- [ ] **CLI-WIREMOCK-502-PREEXISTING: CLI approval wiremock tests fail with 502** `sev-P2`
  2026-08-05 验证 CLARIFY-HITL Phase 1 时 `just test` 命中 6 个既有 CLI 测试失败
  （`approval_commands::tests::*`、`chat_commands::approval_mode_tests::*`），
  报 `502 Bad Gateway`（wiremock mock server 未收到匹配请求）。用
  `git stash` 回到干净树复跑**同样失败**，确认与本迭代变更无关，疑为
  Windows 系统代理/环境干扰 wiremock 本地端口。需独立迭代排查（代理绕过
  或 mock 服务器隔离），并在 `just test` 全绿后关闭。

- [ ] **GATEWAY-PORT-CONFIG-IGNORED: CLI gateway run 忽略 config.gateway.port** `sev-P3`
  2026-08-06 做 GUI provider 错误可见性 smoke 时发现：`config.json` 的
  `gateway.port` 字段已存在，但 `agent-diva-cli/src/main.rs`
  `build_gateway_runtime_config` 硬编码 `port: DEFAULT_GATEWAY_PORT`（3000），
  用户配置的端口不生效。导致无法在隔离端口起第二个 gateway 实例做 HTTP SSE
  smoke（3000 被用户 gateway 占用）。期望行为：`run_gateway` 优先使用
  `config.gateway.port`（回退 DEFAULT）。修复面小（main.rs 一处），独立提交。

- [ ] **GUI-TAURI-PLAN-STREAM-DISCONNECT: plan 流式 Tauri command 断流兜底统一** `sev-P3`
  2026-08-06 修复 `GUI-PROVIDER-ERROR-SILENT` 时只为主 chat 流
  （`commands.rs` `send_message` SSE 循环）加了 `saw_terminal` 断流兜底
  （SSE 无 final/error 结束时 emit `agent-error`）。`commands.rs` 另有
  两个 plan 相关 SSE 循环（L2854、L3140）仍是「流结束静默返回」，异常断开时
  前端可能保持挂起。期望行为：与主 chat 流一致，循环退出无终止事件时
  emit `agent-error`（或等价恢复事件）。本次按计划未动，待独立迭代统一。

- [ ] **PROVIDERS-EXAMPLE-1.94-CLIPPY: pre-existing example clippy lint** `sev-P3`
  `cargo clippy --all-targets -- -D warnings` 命中 `agent-diva-providers/examples/
  minimax_sync_tts.rs:99` `useless_conversion`（Rust 1.94 新 lint；
  `tls.into()` 转为同类型）。该文件本迭代未改动；`just check`/`just ci`
  （`cargo clippy --all`，不编译 examples）不受影响。修复：移除 `.into()` 或
  `#[allow]`，独立小提交。

- [ ] **SANDBOX-SAVE-FIX-DESKTOP-SMOKE: manual GUI smoke for sandbox settings save** `sev-P2`
  2026-08-05 修复了 GUI 沙箱设置保存失败（kebab-case vs snake_case 枚举不匹配，
  `docs/logs/2026-08-sandbox-settings-save-fix/v0.1.0-sandbox-save-fix/`）。自动化门
  （vitest 451 用例 + vue-tsc/vite build）已通过，但真实 Tauri 桌面冒烟（切换模式保存、
  检查 `~/.agent-diva/config.json`、清空 timeout 边界）未在本会话执行，需按
  `acceptance.md` 步骤人工验收后勾选本项。

- [x] **GUI-RUST-1.94-ALL-TARGETS-CLIPPY: clean pre-existing Tauri test lint** `sev-P3`
  `cargo clippy -p agent-diva-gui --all-targets -- -D warnings` 在既有测试构造代码
  `agent-diva-gui/src-tauri/src/lib.rs` 命中 `field_reassign_with_default`。GMH-31 的
  Tauri `cargo check`、前端测试与生产构建均通过；在独立兼容性提交中机械修复，
  不与审批契约功能混合。

- [x] **WORKSPACE-TEST-UNUSED-FIXTURES: clean two test-only warnings** `sev-P3`
  `just test` 在既有 `agent-diva-channels/src/feishu.rs` fixture 的 `handler` 与
  `agent-diva-tools/src/filesystem.rs` fixture 的 `temp_dir` 报告未使用变量。全量测试
  与 `just check` 均通过；在独立测试清理提交中移除或以下划线明确保留。

- [x] **CORE-RUST-1.94-ALL-TARGETS-CLIPPY: clean pre-existing test lints** `sev-P2`
  `cargo clippy -p agent-diva-core --all-targets -- -D warnings` exposes 19
  pre-existing test-target findings across supervised/config/session/audit and
  related modules. The production library target and official `just check`
  pass; repair these warnings in a separate compatibility slice.

- [x] **MANAGER-LOG-RANGE-FULL-SUITE-FLAKE: isolate shared log state** `sev-P2`
  The first 2026-08-03 `just test` run failed
  `handlers::logs::tests::logs_filter_by_range` because an unexpected event
  entered the selected range; the focused rerun passed. Isolate its log source
  or clock/range fixture so workspace parallelism cannot contaminate it.

- [x] **MANAGER-RUST-1.94-ALL-TARGETS-CLIPPY: clean pre-existing test lints** `sev-P2`
  `cargo clippy -p agent-diva-manager --all-targets -- -D warnings` exposes six
  pre-existing test-target findings (`items_after_test_module`, needless borrow,
  `len_zero`, and two `single_match` cases). Production `cargo check` and the
  official `just check` remain the delivery gate; repair these in a separate
  compatibility slice without mixing them into GMH-30B1.

- [x] **CLIPPY-LINES-MAP-WHILE: update fallible line iteration** `sev-P2`
  The run-event reader now propagates line I/O failures explicitly while still
  skipping malformed JSON, with focused invalid-data coverage. Workspace gate
  restoration is verified with the GMH-30A batch.

- [x] **LAPUTA-RECOVERY-RECEIPT-FIXTURE: restore prepared journal recovery test** `sev-P1`
  The fixture now creates a non-expired approval revision and proves prepared
  recovery, one-time receipt consumption, and idempotent replay. Full workspace
  gate restoration is verified with the GMH-30A batch.

- [ ] **WINDOWS-RELEASE-EXEC-ACCESS: restore local release executable launch** `sev-P1`
  **排期决策（2026-08-10）**：稍后处理，本轮不启动。
  The 2026-08-02 Tauri rebuild produced updated EXE/NSIS/MSI artifacts, but
  Windows rejected `Start-Process target/release/agent-diva-gui.exe` with OS
  error 5 (`Access denied`). Diagnose endpoint protection/file policy or the
  post-link hard-link state, then repeat the desktop health smoke.
  2026-08-03 final-candidate audit rebuilt the complete Tauri release successfully;
  source and isolated copies have matching SHA-256, permissive ACLs, no Zone.Identifier,
  and no matching AppLocker/Code Integrity/Defender event, but all ordinary-user launch
  attempts still return OS error 5. Resolving unsigned-binary trust/endpoint policy needs
  explicit human/system-policy authorization; the final manual smoke starts with this gate.

- [x] **跨平台 canonical workspace identity 与 identity-only migration** `sev-P1`
  Windows 路径分隔符差异曾使 Migration 与 Manager 对同一 workspace 计算出不同
  identity，并由 typed fail-closed 检出。统一 CLI/Manager/GUI/Migration 的 canonical
  规则，提供只迁 identity、不复制或改写 Memory 内容的可验证迁移与回滚。
  相关：执行蓝图 B2、GMH-24 Migration/typed authority。

- [x] **QQ invalid-resume 集成测试 load-sensitive** `sev-P2`
  全量 gate 偶发 opcode 乱序；隔离重跑通过。
  `agent-diva-channels/tests/qq_reconnect_integration.rs`。
- [x] **Memory authority provider 选择缺 focused characterization** `sev-P2`
  `.laputa/` 打开失败应选 `DegradedMemoryProvider`；
  `cargo test -p agent-diva-agent memory_boundary` 当前 0 测。
  `agent-diva-agent/src/memory_boundary.rs`。
- [x] **Manager library suite 在 workspace gate 下 load-sensitive** `sev-P2`
  隔离 `cargo test -p agent-diva-manager --lib` 可通过。
  需定位并同步不稳定用例。
- [ ] **Workspace Rust 1.80 MSRV 与无关新依赖冲突** `sev-P2`
  GMH-24 已移除 Mentle 链；仍有 ICU/Darling/Pest/CRC/Tauri 等声明 MSRV >1.80。
  独立 pin/升级切片；勿削弱 clean-break gate。
- [ ] **MSRV 探测污染默认 target cache** `sev-P2`
  未来 `cargo +1.80` 须用独立 `CARGO_TARGET_DIR`。
- [ ] **Laputa service 预存 clippy `int_plus_one`** `sev-P3`
  `agent-diva-laputa/tests/service.rs` 四处断言风格问题。
- [ ] **StepFun 真实 endpoint E2E（model pass-through）** `sev-P3`
  单测已覆盖透传；缺真实 key 时的 E2E。使用桌面 `keys.txt`，勿入库。

### Plan Residual（2026-07-30 重评后仍 open）

对照当前代码保留；完整历史处置表见
[`docs/archive/todolist/plan-todo-p1-p3-disposition-2026-07-30.md`](docs/archive/todolist/plan-todo-p1-p3-disposition-2026-07-30.md)。

- [x] **扩展 phase×capability / transition 矩阵与 denial 副作用测试** `sev-P2`
  policy 已 fail-closed，但缺完整非法迁移笛卡尔与 denial 前后 store 快照断言；
  assembly 仅子集工具。
  相关：`agent-diva-core/src/planning/policy.rs`、
  `agent-diva-agent/src/tool_assembly.rs`、agent-loop 集成夹具。
- [x] **runtime 配置热更新后按 phase 重建工具表** `sev-P2`
  `rebuild_tools_for_active_phase` 当前 `rebuild_tools_for_turn(..., None, ...)`，
  网络/MCP 更新可能短暂丢掉 phase 边界。
  `agent-diva-agent/src/agent_loop/loop_runtime_control.rs`。
- [x] **空 execution TODO 列表的 Verify 门闩** `sev-P2`
  `PlanVerifier::verify` 在 `total == 0` 时直接 Pass，可能让无步骤计划误完成。
  `agent-diva-agent/src/planning/verifier.rs`。
- [x] **预存在 TODO 与 materialize 策略文档化/修复** `sev-P2`
  `TodoAlreadyMaterialized` 仍可永久卡住 Always 路径；需产品决策：视为已物化
  成功、禁止预批准写入、或提供清理 API。
  `agent-diva-core/src/planning/store.rs`。
- [x] **删除 orchestrator 内注释掉的死迁移矩阵** `sev-P3`
  `agent-diva-agent/src/planning/orchestrator.rs` 大段注释旧表；core 已是唯一真相。

### Deferred Product

- [ ] **CLARIFY-HITL：ask_user 对话询问闭环** `sev-P1`
  **排期决策（2026-08-10）**：稍后处理，本轮不启动。
  2026-08-05 用户复现：期望 Agent 用「询问工具」做互动调研，Agent 正确声明无表单/
  问卷工具并退化为纯文本。根因是**能力缺失**而非模型偶发未调用：无 `clarify`/
  `ask_user` 类工具；`MessageTool` 未接入 `ToolAssembly`；system prompt 强制正常对话
  直接回文本；无「提问→挂起→用户答→tool result」运行时与 GUI/CLI 问题卡。
  **与 M3 审批 HITL（GMH-30..33）分轨**：不得并入 governance.db / 审批抽屉。
  权威提案：
  [`docs/research/ask-user-clarify-hitl-proposal.md`](docs/research/ask-user-clarify-hitl-proposal.md)；
  日志：
  [`docs/logs/2026-08-ask-user-hitl-research/`](docs/logs/2026-08-ask-user-hitl-research/)。
  **进度**：Phase 1 运行时 MVP 完成（`956bdd66`）；Phase 2 表面闭环完成
  （`2b6283c8` manager API、`be43c130` CLI interactive、`342807a1` Tauri 桥、
  `cf268e5e` GUI QuestionCard）——仅剩人工 smoke（真实 LLM 触发 ask_user 的
  CLI/GUI 验收，步骤见 `v0.2.0-ask-user-surface/acceptance.md`）。**已拍板
  （2026-08-05）**：工具名 `ask_user`、单题单轮、挂起默认超时 10 分钟。
  对照：Hermes `clarify`、Claude Code `AskUserQuestion`、OpenHarness
  `ask_user_question`。实现注意：`AskUserTool` 必须实现 `timeout_secs()` 覆盖
  registry 全局 120s 超时（`agent-diva-tooling/src/registry.rs:41`、`base.rs:18`）。
  Phase 3 待办：Plan 矩阵细化、subagent 黑名单断言、UI 与审批抽屉区分、
  可选 messaging clarify。
  **2026-08-08 增补**：① `allow_other` 默认翻转 true（`d303b12a`）——省略
  `choices` 即为开放式填空，提供 `choices` 自动带 Other 兜底，对齐 Hermes
  `clarify` 语义；② 首次引导注入（`96fea09c`）——Frozen Core 为空时 system
  prompt 注入 `First-Run Onboarding` 块，指示用 `ask_user` 收集身份偏好；③
  `laputa_propose_section_write` 工具（`1a70b1ae`）——引导答案转为 governed
  proposal 落 Frozen Core（identity/relationship/commitment/preferences）。

- [ ] **Skill 可视化全生命周期编辑器（延期）** `sev-P3`
  产品对象只有 Skill；SOP 不建立独立类型、标签、入口、DTO、存储或运行时，
  “SOP”仅是用户对某类 Skill 内容的称呼。未来若启动，统一为全部 Skill 提供
  可视化创建、读取、编辑、删除、校验、预览与安全审查，而不是只做 SOP 编辑器。
  当前不在预期范围，须由用户明确恢复后再设计/实现。
  决策：`docs/architecture/skill-sop-unification.md`。

- [ ] **Mask feature 验收** `sev-P2`
  `.sisyphus/plans/mask-feature-implementation.md` 剩余项，待独立恢复。
- [ ] **Wave 3 residual：enqueue_background_task 生产路径 E2E 证明** `sev-P2`
  `agent-diva-tools/src/enqueue_background_task.rs`、assembly / agent_loop。
- [ ] **Wave 3 residual：supervised subagent worker bootstrap 刻画** `sev-P2`
  启动/排空/取消/重启缺生产路径测试。
- [ ] **Wave 3 residual：subagent 终态生命周期 E2E** `sev-P2`
- [ ] **Wave 3 residual：background task 上下文与预算继承 E2E** `sev-P2`
- [ ] **Wave 3 residual：workspace CLI managed-path 与任意路径产品契约** `sev-P3`
  路径穿越已修；managed `config_dir/workspaces/*` 与 runtime 任意路径模型仍分歧。
- [ ] **Approval dual-channel unification + 超时 UX** `sev-P3`
  Legacy `command-approval-requested` SSE（`App.vue:2003`）与统一 `approval-event`
  （`App.vue:2006`）两套通道并存，GUI 同时维护 `commandApprovals` 与 `unifiedApprovals`，
  容易丢事件。同时 `ApprovalCoordinator` 默认超时后直接 `Expired`，前端无倒计时，
  用户无法感知剩余审批窗口。2026-08 cautious-mode 修复时仅修了后端透传+escalation，
  通道合并与倒计时待独立迭代。
  关联：`docs/logs/2026-08-cautious-approval-fix/v0.5.0-cautious-approval/`.
- [ ] **审批三模式完善（对齐 Claude Code 三窗口）** `sev-P2`
  GUI「智能/谨慎/信任」三模式后端行为趋同，属半残：
  (1) 信任≡谨慎（`guardian.rs:369` OnRequest|UnlessTrusted 同分支、`exec_policy.rs:299` 同处理）；
  (2) 智能=盲跑（`guardian.rs:365` OnFailure 直接 Defer，无风险预判）；
  (3) 自动放行开关生产默认全关（`orchestrator.rs:885` 固定 `GuardianConfig::default()`）；
  (4) 模式不持久化（`ChatView.vue:191`）。
  完善方案：谨慎→全问+strict；智能→只读/known-safe 自动放行+危险询问；信任→未知放行+危险询问+自动学习。
  详见 `docs/research/approval-model-claude-code-vs-agent-diva.md` §4-§5。
- [x] **审批 UI 三重显示去重** `sev-P2`  *(v0.5.1 已修)*
  同一 ExecTool 审批请求在 GUI 同时出现三种视觉形态：
  (1) Drawer 内的 `ApprovalCenterCard`（完整样式，`ApprovalCenterDrawer.vue:155`）；
  (2) Chat 消息流底部的内联 `ApprovalCenterCard`（compact，`ChatView.vue:1051`）；
  (3) 偶尔闪现的 legacy `ApprovalBanner`（`ChatView.vue:1068`）。
  根因：后端 `approval_coordinator` 同时广播 legacy `command_approval_requested`
  与写入 governance ledger，前端互斥守卫只盖 legacy Banner。
  修复方案：单通道 + 单渲染点（保留 Drawer 为唯一全量入口，删除 Chat 内联两份重复渲染），
  详见 `~/.qoder-cn/plans/daring-dune-thrush.md`（2026-08 修订版）。
  关联：`docs/logs/2026-08-cautious-approval-fix/v0.5.1-approval-ui-dedup/`.
- [ ] **UX-DR-3/4/7** `sev-P3`
  Sprint 评审 UX 缺口，待专项设计。
- [ ] 待任务排期

---

## Archive Index

| 文档 | 内容 |
|------|------|
| [`docs/archive/todolist/completed-through-2026-07-29.md`](docs/archive/todolist/completed-through-2026-07-29.md) | 早期完成项与 P2/P3 boundary 实现索引 |
| [`docs/archive/todolist/completed-through-2026-07-30.md`](docs/archive/todolist/completed-through-2026-07-30.md) | G0、GMH-00..24、Deferred Review Program 关闭、G1 等 |
| [`docs/archive/todolist/plan-todo-p1-p3-disposition-2026-07-30.md`](docs/archive/todolist/plan-todo-p1-p3-disposition-2026-07-30.md) | 2026-07-11 Plan P1–P3 评审逐条 CLOSED/SUPERSEDED/OBSOLETE/OPEN-RESIDUAL |
| [`docs/logs/2026-07-todolist-triage/v0.0.1-archive-and-residual/`](docs/logs/2026-07-todolist-triage/v0.0.1-archive-and-residual/) | 本次瘦身迭代日志 |

**原则：** 主清单只保留本阶段 Active Plan 与真实未完成项；完成项迁 archive，不静默删除。
