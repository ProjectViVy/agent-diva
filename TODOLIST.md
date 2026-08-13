# TODOLIST

项目级**活跃**待办。这里只保留仍可执行、仍待决策或仍需验证的事项；完成、取消、
被取代和重复记录统一进入 [`docs/archive/todolist/`](docs/archive/todolist/README.md)。

严重度：`sev-P0` 阻断，`sev-P1` 高，`sev-P2` 中，`sev-P3` 低。

## 总 EPIC：Laputa 认知工作区 Clean Break

- [ ] **LAPUTA-COGNITIVE-WORKSPACE-RESET：完成研究、架构评审、破坏性重构与纵向验收** `sev-P0`
  当前状态：`Epic Defined / R0–R4 Packages Complete / Research Gate Pending / Architecture Design Blocked`。总 EPIC 统一编排
  Persona/WORLD、Memory/BML、跨会话 STM、Evolution/Skill 与聊天 Approval Center；目前
  只授权 R0–R4 调研，**研究评审完成前不得定稿新架构，架构评审完成前不得修改生产代码**。
  总编排与门禁：
  [`docs/research/cognitive-workspace-reset-epic-2026-08/epic-orchestration.md`](docs/research/cognitive-workspace-reset-epic-2026-08/epic-orchestration.md)。

**已冻结产品边界（不是待重新决策项）：**

- Memory CRUD 与 STM 维护不走审批；BML 是普通长期 Memory 唯一权威；完整删除
  `MemoryMd` / `memory_md` 链路且不自动导入旧数据。
- Persona 是四份核心 Markdown + WORLD，不等于 Memory；用户直接保存，Agent 建议只走
  Persona 专属 before/after 内容审查；首次初始化 absence-only 原子直写五份权威；所有
  Persona/WORLD 历史永久保留。
- Evolution 只管理 Skill；旧 AutoDream Evolution 不再修补；SOP/Skill 最终关系由最新
  GenericAgent 研究决定。
- Chat Approval Center 只保留危险运行时操作等真正授权；旧领域治理不得回流。
- 全部重构采用 clean break；实施前从验证提交建立保护性分支，保护分支不作为 fallback。

决策依据：
[Evolution](docs/research/evolution-genericagent-reset-2026-08/decision-record.md)、
[Persona](docs/research/persona-markdown-clean-break-2026-08/decision-record.md)、
[STM/Memory](docs/research/stm-cross-session-clean-break-2026-08/decision-record.md)。

### Research Gate：现在允许执行

- [x] **COGNITIVE-R0-CURRENT-STATE：当前系统、数据与耦合全景盘点** `sev-P0`
  **研究包已交付（2026-08-13）**，待用户 Research Gate：
  [`docs/research/cognitive-r0-current-state-2026-08/README.md`](docs/research/cognitive-r0-current-state-2026-08/README.md)。
  三份完成物：当前状态图、依赖/数据清单（KEEP/RENAME/DELETE/DECIDE）、旧架构失败基线。
  核心事实：混域 Proposal 链、Persona JSON↔BML 双权威、`memory_md` 仍活、
  `governance.db`/`governance.sqlite3` 双账本、事件四轨。不沿旧模型打补丁。
  R1 Evolution 切片仍有效，由全量 R0 引用。

- [x] **COGNITIVE-R1-GENERICAGENT-EVOLUTION：更新并研究 GenericAgent Evolution** `sev-P0`
  **研究包已交付（2026-08-13）**，待用户 Research Gate：
  [`docs/research/cognitive-r1-genericagent-evolution-2026-08/README.md`](docs/research/cognitive-r1-genericagent-evolution-2026-08/README.md)。
  本地 GA 锁定 `ee5a474`；远程 tip（API）`f06d550`；触发/Action-Verified/L0–L4/SOP-Skill/
  实验/差距/建议齐全。P1 活体实验因无 `mykey.py` 阻断，记入开放项。研究完成前仍禁止
  实现晋升状态机与 D3 架构定稿。

- [ ] **COGNITIVE-R1b-GA-LIVE-EXPERIMENT：GenericAgent 活体结晶实验（可选补强）** `sev-P2`
  在具备隔离密钥与 temp 记忆沙箱时，测量 Action-Verified 遵守率、未验证写入率、
  L1 行数违规与 patch/overwrite 比；补强 `evolution-behavior-experiments.md` P1/P2。

- [x] **COGNITIVE-R2-STM-CONTEXT：STM 与上下文分层专项研究** `sev-P0`
  **研究包已交付（2026-08-13）**，待用户 Research Gate：
  [`docs/research/cognitive-r2-stm-context-2026-08/README.md`](docs/research/cognitive-r2-stm-context-2026-08/README.md)。
  三份完成物：装配约束、Hold 选项与静态实验、失败/并发矩阵。产品 STM 不存在；
  CanonicalCheckpoint / SessionCheckpoint / BmlStartupIndex 已拆名。未选物理权威。

- [x] **COGNITIVE-R3-PERSONA-WORKSPACE：Persona 文档权威与工作区技术研究** `sev-P0`
  **研究包已交付（2026-08-13）**，待用户 Research Gate：
  [`docs/research/cognitive-r3-persona-workspace-2026-08/README.md`](docs/research/cognitive-r3-persona-workspace-2026-08/README.md)。
  三份完成物：权威盘点、revision/Diff/CAS 选项、Markdown 工作区技术评估。JSON
  section + Proposal 仍是权威；`content_version` 只展示；changelog ≠ 文档历史；
  WORLD `project()` 无生产 Prompt 调用者。未选目录 / Diff 引擎 / CM6 扩展。

- [x] **COGNITIVE-R4-CLEAN-BREAK-SAFETY：数据影响、保护分支与删除证明研究** `sev-P0`
  **研究包已交付（2026-08-13）**，待用户 Research Gate：
  [`docs/research/cognitive-r4-clean-break-safety-2026-08/README.md`](docs/research/cognitive-r4-clean-break-safety-2026-08/README.md)。
  三份完成物：删除影响、保护分支协议、零残留证明目录。未抽样生产 profile；
  未创建保护分支；当前 tip 不是删除前基线。禁止导入/fallback。

### Architecture Gate：全部 Research 通过用户评审后才能开始

- [ ] **COGNITIVE-D0-DOMAIN-AUTHORITY：总体领域、权威、生命周期与禁止依赖设计** `sev-P0` `blocked:R0-R4`
  为 Persona/WORLD、BML、STM、SessionCheckpoint、Evolution/Skill、Approval 形成单一权威
  和跨域不变量 ADR；当前只登记设计任务，不写目标方案。

- [ ] **COGNITIVE-D1-PERSONA：Persona/WORLD/首次初始化/历史架构设计** `sev-P0` `blocked:D0`
  研究通过后设计 Markdown 权威、revision/Diff、直接保存、专属内容审查、Frozen Core、
  原子初始化、incomplete repair、永久历史及 Manager/Tauri/GUI 契约。

- [ ] **COGNITIVE-D2-MEMORY-STM：Memory/BML/STM/Context 架构设计** `sev-P0` `blocked:D0`
  研究通过后设计 BML CRUD、STM 权威/自动化、SessionCheckpoint 分离、Layer 1 装配、
  并发失败恢复、历史和 Memory/STM GUI。

- [ ] **COGNITIVE-D3-EVOLUTION-SKILL：Evolution/SOP/Skill 架构设计** `sev-P0` `blocked:D0`
  研究通过后设计唯一 Evolution 领域模型、触发、验证、可管理性、SOP/Skill 关系、Skill
  runtime 接线和 GUI；人格及普通 Memory 必须排除。

- [ ] **COGNITIVE-D4-CLEAN-BREAK-DELIVERY：接口、删除、发布与验收设计** `sev-P0` `blocked:D1-D3`
  汇总窄 API/事件/错误、实施依赖、删除矩阵、保护分支、提交切片、发布/恢复、自动测试和
  真机验收。D4 用户批准后才允许领取生产代码范围。

### Implementation Gate：架构批准后再展开

- [ ] **COGNITIVE-I0-PROTECTION-BASELINE：建立并验证删除前保护性分支** `sev-P0` `blocked:D4`
  从 D4 指定且验证通过的准确提交创建；不提前创建，不作为兼容 runtime。

- [ ] **COGNITIVE-I1-CLEAN-BREAK-IMPLEMENTATION：按批准设计分切片实施并验证** `sev-P0` `blocked:I0`
  具体文件、顺序和提交数量等待 D4 决定；要求每片独立验证、独立 Conventional Commit，
  最终执行旧符号/路由/数据/GUI/Prompt 零残留证明、全仓 gate、真实桌面 smoke 和恢复演练。

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
