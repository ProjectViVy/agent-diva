# TODOLIST

项目级**活跃**待办。这里只保留仍可执行、仍待决策或仍需验证的事项；完成、取消、
被取代和重复记录统一进入 [`docs/archive/todolist/`](docs/archive/todolist/README.md)。

严重度：`sev-P0` 阻断，`sev-P1` 高，`sev-P2` 中，`sev-P3` 低。

## 当前决策门

- [ ] **EVOLUTION-GENERICAGENT-RESET-RESEARCH：Evolution 非兼容重置专项调研** `sev-P0`
  先更新并逐提交审查上层 `.workspace/GenericAgent` 主分支，研究最新自进化触发、
  Action-Verified 沉淀、L0–L4、SOP/Skill 产物、发现复用和可管理性；同时盘点 Diva
  现有 AutoDream、Laputa proposal、Memory governance、Skill、Manager/Tauri、GUI、
  持久化和测试的完整依赖。输出单一领域模型建议、失败模型、非兼容删除清单、数据处置、
  测试矩阵和真实桌面验收方案。实施前必须从删除前已验证提交建立保护性分支；保护分支
  不得成为兼容 runtime。已拍板约束：旧 AutoDream–Evolution 链路不再修补；Evolution
  不管理人格或普通记忆；SOP/Skill 最终关系继续待调研，不提前实现晋升状态机。
  决策：
  [`docs/research/evolution-genericagent-reset-2026-08/decision-record.md`](docs/research/evolution-genericagent-reset-2026-08/decision-record.md)。

- [ ] **MEMORY-APPROVAL-CLEAN-BREAK：Memory CRUD 完全退出审批体系** `sev-P0`
  Memory 的增、删、改、查直接作用于 Memory 权威，不创建 Laputa Proposal，不进入
  Evolution 或 Governance Ledger。实施时非兼容删除 Memory 专属审批、proposal-first
  写入、治理投影、旧 DTO/状态映射和 fallback，并以精确目标、软删除、历史版本、撤销
  或恢复承担误操作保护。聊天页统一 Approval Center 仅保留危险工具执行等运行时授权。
  与 Evolution 重置共享依赖盘点，但代码删除和验证使用独立切片与提交。

- [ ] **PERSONA-MARKDOWN-CLEAN-BREAK：人格 Markdown 权威与工作区重构** `sev-P0`
  将 Identity、Relationship、Commitment、Preferences 的权威正文、版本、Diff、历史、
  回滚、Frozen Core 快照和 Prompt 投影全部改为 Markdown 字符串。GUI 重构为人格导航、
  单一中央工作区及“当前文档 / 待审变更 / 历史”三态；用户手动编辑直接保存并产生审计。
  Agent/系统建议使用 Persona 专属 Markdown 内容审查：只读 before/after Diff、原子接受
  或拒绝、base revision 失配即 stale；它不进入聊天 Approval Center，也不复用通用
  Governance，且不拆分 approve/apply。历史版本只读，载入仅覆盖本地草稿，显式保存后
  才成为新当前版本。删除永久右栏、人格 `.json`、JSON patch/format/parse、旧
  SOUL/IDENTITY/USER 映射、迁移、双读写和 fallback。
  实施前建立删除前保护性分支，不自动导入旧用户数据。HTTP/Tauri 可继续用结构化信封，
  但人格正文和用户界面不得再出现 JSON。决策：
  [`docs/research/persona-markdown-clean-break-2026-08/decision-record.md`](docs/research/persona-markdown-clean-break-2026-08/decision-record.md)。

## 产品与架构

- [ ] **PLAN-MODE-PHYSICAL-STATE-MACHINE：Plan Mode 物理限制状态机** `sev-P1`
  从已完成的 Context 主线分轨，需另行冻结状态、能力矩阵和非法迁移验收。

- [ ] **EVENTBUS-TRAIT-HOOKS：EventBus Trait Hook 管道** `sev-P1`
  来源于 OpenHarness 调研；保留为未来扩展点，当前延期。

- [ ] **WORLD-MEMRULES-GATE：WorldGovernance submit 阶段 MemRules R6 拦截** `sev-P2`
  在 `WorldGovernance::submit` 加载 MemRules，违反 R6 时返回稳定 protected 原因；
  必须保持 WORLD/人格权威与普通 Memory CRUD 的新边界，不得借此恢复 Memory 审批。

- [ ] **MEMRULES-DEFAULT-SEED-ALIGNMENT：默认 R1–R7 与生产策略对齐** `sev-P3`
  复核 `DEFAULT_MEM_RULES_TEXT` 与 `PolicyRestriction`、autonomy level 的语义；必要时
  调整措辞或新增规则。

- [ ] **RG-CODE-GOV 后续分期** `sev-P2`
  原位治理 G0/G1 已完成；剩余 G2 Manager handler 变薄、G3–G5 GUI Host/state/DTO。
  不回迁 deep-governance 大爆炸。设计：`docs/dev/agent-loop-manager-gui-governance/`。

- [ ] **全仓库代码清理提案审批与排期决策** `sev-P3`
  对 `docs/logs/2026-08-05-code-audit-proposal/v0.1.0-code-audit-proposal/` 的清理集合
  做取舍；不得把已被 Evolution/Memory 新决策覆盖的旧方案重新实现。

- [ ] **GMH-53：灰度与清理** `sev-P2`
  只处理仍有效的通用治理/发布内容；Memory governance 与旧 Evolution 部分由上述
  clean-break 决策取代。

- [ ] **day/hour token 死循环熔断安全阀决策** `sev-P3`
  定位为极高默认阈值的异常循环保护，而不是常规预算管理；决定全局或会话窗口，以及
  与 `session_token_budget_limit`、`RejectionCircuitBreaker` 的交互后再实施。

- [ ] **Workspace CLI managed-path 与 runtime 任意路径契约** `sev-P3`
  路径穿越已修；仍需统一 `config_dir/workspaces/*` 与 runtime 任意路径模型。

- [ ] **CLARIFY-HITL Phase 3** `sev-P3`
  已有 `ask_user` 运行时、CLI/Tauri/GUI 表面；剩余 Plan 矩阵、subagent 禁用断言与
  可选 messaging clarify。不得并入审批抽屉或 governance ledger。

- [ ] **UX-DR-3/4/7** `sev-P3`
  Sprint 评审遗留 UX 缺口；恢复前先重新确认原问题仍存在并补专项设计。

## 自动化与生产路径证明

- [ ] **BACKGROUND-TASK-PRODUCTION-E2E：后台任务生产路径纵向证明** `sev-P2`
  覆盖 `enqueue_background_task` 的 assembly/agent loop 接线、supervised worker
  启动/排空/取消/重启、subagent 终态，以及上下文与预算继承。

- [ ] **STEPFUN-REAL-ENDPOINT-E2E：StepFun model pass-through** `sev-P3`
  单测已覆盖 model 透传；仍需使用桌面 `keys.txt` 做真实 endpoint E2E，密钥和未脱敏
  响应不得进入仓库、日志、夹具或提交。

## 人工与真实桌面验收

- [ ] **M3-HITL-DESKTOP-SMOKE：审批三模式集中验收** `sev-P2`
  验证谨慎/智能/信任三模式、trusted 规则学习、重启保持和危险 shell 仍受 Guardian
  限制。审批只从聊天页统一 Approval Center 出现；不包含 Memory CRUD 或旧 Evolution。
  参考：`docs/logs/2026-08-m3-hitl-closure/`。

- [ ] **WINDOWS-RELEASE-EXEC-ACCESS：恢复本地 release 可执行启动** `sev-P1`
  当前普通用户启动 release EXE 返回 Windows OS error 5；需明确的人工/系统策略授权，
  解决后再执行 M3 与其他真实桌面 smoke。

- [ ] **SANDBOX-SAVE-FIX-DESKTOP-SMOKE** `sev-P2`
  切换沙箱模式并保存，检查配置落盘及清空 timeout 边界。步骤：
  `docs/logs/2026-08-sandbox-settings-save-fix/v0.1.0-sandbox-save-fix/acceptance.md`。

- [ ] **CLARIFY-HITL-DESKTOP-SMOKE** `sev-P2`
  用真实 LLM 触发 `ask_user`，分别验证 CLI/GUI 提问、回答、超时和恢复。步骤：
  `docs/logs/2026-08-ask-user-hitl-research/v0.2.0-ask-user-surface/acceptance.md`。

- [ ] **GATEWAY-PORT-DESKTOP-SMOKE** `sev-P3`
  配置 `gateway.port` 后确认 gateway 监听配置端口；未配置时仍使用 3000。步骤：
  `docs/logs/2026-08-small-fixes-batch/v0.5.1-small-fixes-batch/acceptance.md`。

- [ ] **GUI-PROVIDER-ERROR-RETRY-DESKTOP-SMOKE** `sev-P2`
  用真实失败/重试场景确认 GUI 显示重试进度、stall 和最终错误，不因 request ID 或
  SSE 断流永久挂起。参考：`docs/logs/2026-08-gui-provider-error-visibility/`。

- [ ] **MASK-FEATURE-ACCEPTANCE** `sev-P2`
  恢复 `.sisyphus/plans/mask-feature-implementation.md` 前先复核现状，再执行剩余验收。

## Reliability / Test Debt

- [ ] **LAPUTA-STORAGE-STALE-LOCK-FLAKE** `sev-P2`
  Windows 全工作区负载下 stale lock 回收偶发 `LockTimeout`，定向重跑与后续 CI 通过；
  需隔离临时目录锁回收时序。关联 `agent-diva-laputa/src/lock.rs`。

- [ ] **WORKSPACE-GUI-TOOLING-LOAD-FLAKES** `sev-P2`
  GUI embedded gateway 启动和 tooling registry timeout 测试曾在 full suite 偶发失败、
  focused 重跑通过；隔离共享资源和时序依赖。

- [ ] **CLI-WIREMOCK-502-PREEXISTING** `sev-P2`
  CLI approval wiremock 用例在 Windows 环境偶发/持续返回 502；排查系统代理绕过与 mock
  服务器隔离，关闭标准是 `just test` 全绿。

- [ ] **GUI-TAURI-PLAN-STREAM-DISCONNECT** `sev-P3`
  两条 Plan SSE/Tauri 循环仍可能在无终止事件断流时静默返回；统一为明确错误或恢复事件。

- [ ] **AGENT-DIVA-FILES-CLIPPY** `sev-P3`
  修复 `agent-diva-files/src/s3.rs` 在 Rust 1.94 下的
  `empty_line_after_doc_comments`，使用独立机械提交。

- [ ] **WORKSPACE-MSRS-1.80-DEPENDENCY-CONFLICTS** `sev-P2`
  工作区声明 Rust 1.80，但 ICU/Darling/Pest/CRC/Tauri 等依赖存在更高 MSRV；需要独立
  pin/升级方案，不削弱现有 gate。

- [ ] **MSRV-ISOLATED-TARGET-CACHE** `sev-P2`
  所有 `cargo +1.80` 探测必须使用独立 `CARGO_TARGET_DIR`，避免污染默认 target cache；
  将此约束固化进验证命令或脚本。

- [ ] **LAPUTA-TESTS-1.94-ALL-TARGETS-CLIPPY** `sev-P3`
  `cargo clippy -p agent-diva-laputa --all-targets -- -D warnings` 仍有测试目标 dead code、
  `cmp_owned` 等 lint；生产库目标和 `just check` 不受影响，独立机械修复。

## Archive Index

- [`docs/archive/todolist/README.md`](docs/archive/todolist/README.md)
- 清理前完整快照：
  [`snapshot-before-2026-08-13-cleanup.md`](docs/archive/todolist/snapshot-before-2026-08-13-cleanup.md)
