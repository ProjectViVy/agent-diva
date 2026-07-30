# Agent Diva 全量待办执行蓝图

- 状态：`Accepted for planning`
- 日期：2026-07-30
- 适用范围：根 `TODOLIST.md` 中全部活跃事项
- 实施授权：本文只定义顺序、边界和门控，不授权本次修改运行时代码

## 1. 目标与完成定义

本蓝图用于让 Codex“目标”功能能够持续推进待办，而不是把清单机械地从上到下
执行。项目完成必须同时满足：

1. 根 `TODOLIST.md` 没有未处置的 `sev-P0/P1`，`sev-P2/P3` 均已完成、明确
   取消或由用户书面接受延期；
2. typed Laputa 是唯一生产 Memory authority，不存在静默 legacy fallback；
3. Plan、Sandbox、Memory 的审批、receipt、审计和恢复共享一致治理语义；
4. AgentLoop 的所有副作用都经过单一治理 seam，Ask、Mask、子代理、cron
   不可提升权限；
5. GUI、CLI、Manager/Tauri 的契约和失败表现一致；
6. Evolution 的 AutoDream–Laputa 纵向链路达到开箱可用，不以旧文档、占位候选或
   不可达入口宣称可用；
7. Rust 1.80、完整测试、真实桌面 smoke、恢复演练和发布清理全部通过；
8. 每个完成切片都有四件套日志、聚焦提交和可回滚证据，且未擅自 push。

“所有 TODO 完成”不包括无限扩张产品范围。新发现项必须先分类和排期；与当前
目标无关的愿望不能自动进入实施。

## 2. 不可变架构原则

- 单 authority：每类状态只有一个生产真相源；兼容层必须有删除日期。
- Fail closed：未知模式、损坏 store、身份不匹配、过期/撤销 receipt 和不确定
  恢复状态都拒绝执行。
- 副作用集中：工具展示不是安全边界，最终执行 seam 必须再次检查。
- 精确授权：批准绑定 request、digest、version、policy、capability 和 resource。
- 可恢复：持久化操作必须具有幂等键、阶段状态、审计链和重启恢复测试。
- Payload-free observability：诊断可包含 ID、摘要、计数、原因码和延迟，不含
  Memory 原文、密钥或未脱敏用户数据。
- 不重建第二套系统：SOP 不成为产品类型，相关编辑能力归入统一 Skill 管理；
  RG-CODE-GOV 不回迁 deep-governance；
  Mentle/LLVM 运行链不得恢复。

## 3. 工作流与硬依赖

```text
B0 AutoDream–Laputa 产品真实基线
  → B1 Experience Journal
  → B2 可恢复 AutoDream Orchestrator
  → B3 Reflection / Candidate Gate
  → B4 Proposal / 统一 HITL（GMH-30..33）
  → B5 typed apply / Recall feedback / 单一副作用 seam（GMH-40..42）
  → B6 一体化 Evolution GUI 与结构治理
  → B7 数据身份、恢复、发布与全自动纵向 E2E
  → B8 G2D+ 真实桌面最终验收与清理
```

人工 G2D 不再是实现前置门：当前提案底座虽可验，但 AutoDream 到 Recall 的产品链
尚未完成，提前反复人工点击不能证明“拿到手就能用”。B0–B7 的自动化安全、数据
完整性和恢复门不得降低；只有全部通过后才进入 B8 最终人工验收。

## 4. 批次设计

### B0–B8：AutoDream–Laputa 开箱可用闭环

本蓝图的产品主线由
[`docs/dev/autodream-laputa-product-closure/`](../dev/autodream-laputa-product-closure/)
替代原“先 G2D、再重新盘点”的顺序。以该目录的 13 部分计划为实施契约：

- B0：characterization 和不可用/degraded 真状态；
- B1：脱敏 Experience Journal；
- B2：持久化队列、阶段状态机和生产 worker；
- B3：provider-neutral Reflection 与 Candidate Gate；
- B4：确定性 ProposalPublisher 和统一审批；
- B5：canonical typed apply、Recall 与效果反馈；
- B6：一体化 Evolution Workspace，清理假入口；
- B7：workspace identity、恢复演练、发布门和全自动纵向 E2E；
- B8：原 G2D 六场景加完整“任务到 Recall”第七场景。

退出条件：真实任务产生可解释候选，经人工治理唯一写入 typed authority，后续会话
可 Recall，回滚后不可 Recall；并发、重启、损坏和拒绝均有确定行为。

人工暂停点：只在 B8 最终桌面验收、真实 provider smoke、密钥、发布或不可逆迁移时
暂停。B0–B7 不再要求用户逐场景点击。

### 原 B1 状态

“重新盘点”已被 B0 characterization 吸收；不再把盘点作为长期停留阶段。发现
`HIDDEN / REMOVE / REDESIGN` 的入口必须在相应纵向切片内完成处置，不能只写报告。

### 横切基础：数据身份与可靠性

优先处理会污染后续验收可信度的基础债：

1. 统一跨平台 canonical workspace identity，并提供仅身份迁移与回滚；
2. 补 Memory provider fail-closed focused characterization；
3. 修复 QQ、Manager load-sensitive 测试并区分产品失败与负载噪声；
4. 为 MSRV 探测使用独立 target cache；
5. 清理已知 Laputa clippy 债。

退出条件：同一 profile 在 CLI、Manager、GUI、Migration 中解析为同一 identity；
完整 gate 不依赖“隔离重跑才算通过”。

### 横切治理：统一 HITL（GMH-30..33）

推荐拆分：

1. GMH-30A 领域协调器与状态机；
2. GMH-30B receipt consumption、恢复、取消与超时；
3. GMH-31 Manager API/SSE reason code 与事件序列；
4. GMH-32A GUI 决策中心；
5. GMH-32B 就地审批、重连去重与无障碍状态；
6. GMH-33 CLI/headless fail-or-queue E2E。

必须覆盖 approve/edit/reject/cancel/stale/revoked/expired/concurrent/restart。
执行器不得接受布尔型“已批准”捷径。

### 横切治理：AgentLoop 单一副作用 seam（GMH-40..42）

先修复现有 Plan residual，再接统一治理：

- 完整 phase×capability 与非法 transition 矩阵；
- 配置热更新保留 active phase；
- 空 execution TODO 不得误判完成；
- 决定 `TodoAlreadyMaterialized` 的唯一产品语义；
- 删除 orchestrator 死迁移矩阵；
- 对 assembly、pre-call、execution 使用同一不可变 turn snapshot；
- 加入预算、熔断、取消点与 payload-free correlation 指标。

退出条件：任何调用路径，包括 Ask、Mask、cron、background task 和 subagent，
均无法绕过 seam 或继承更高权限。

### 闭环后的其他产品能力

只有 AutoDream–Laputa E0–E7 和 G2D+ 完成后才进入，默认建议：

1. Mask 独立验收；
2. background task / supervised subagent 四个 Wave 3 E2E；
3. workspace CLI managed-path 契约；
4. UX-DR-3/4/7；
5. StepFun 真实 endpoint E2E（仅在用户提供环境并授权时）。

每一项都必须先有可观察的用户旅程，再实现内部能力。真实 API 不得作为无人值守
目标的默认步骤。

Skill 可视化全生命周期编辑器不属于默认 B5 路线。只有用户明确恢复该延期项后，
才允许先做产品设计，再把它加入新的产品批次。

### 横切结构治理

RG-CODE-GOV 只在对应模块已有行为 characterization 后实施：

- G2 Manager handler/service；
- G3 GUI API/Host；
- G4 GUI state/composables；
- G5 DTO contract、兼容壳和 dead path 清理。

GMH seam 与 RG-CODE-GOV 重叠时，先完成 GMH 行为契约，再做纯结构重构；
行为修复与结构变更不得混在同一提交。

### 横切发布工程与恢复

- 独立解决 Rust 1.80 ICU/Darling/Pest/CRC/Tauri 依赖；
- 完成 GMH-50 feature flag/迁移清理，不恢复 Mentle 或长期双写；
- 完成 GMH-51 安全、备份、损坏、断电、回滚和恢复演练；
- 校验安装、升级、降级拒绝、Windows service、CLI、GUI/Tauri；
- 建立发布候选版本、变更说明、已知限制和可逆回滚步骤。

### 最终全量验收与清理

- GMH-52 全量 gate：fmt、clippy、test、deletion-proof、GUI tests/build、
  Tauri check、真实桌面 smoke；
- 性能基线：10k Memory、长会话、SSE 重连、并发审批和 background queue；
- 安全基线：路径逃逸、权限提升、receipt replay、日志 payload 扫描；
- GMH-53 灰度、遥测观察、兼容壳/feature flag/死代码删除；
- 同步用户文档、架构状态、TODOLIST archive 和最终 acceptance。

## 5. 目标功能执行协议

目标功能每次只领取一个“可提交切片”，并遵循：

1. 读取 `AGENTS.md`、`LOCK.md`、本蓝图和当前 `TODOLIST.md`；
2. 若存在 `sev-P0`，不得跳到低优先级；
3. 建立精确锁；共享工作树有重叠锁时停止或使用隔离 worktree；
4. 先做只读事实核验，再更新计划；
5. 一个切片对应一个 concern、一个日志目录、一个 Conventional Commit；
6. 发现新问题立即加入 TODO，并标明是否阻断当前批次；
7. 验证失败时修复或记录真实 blocker，不得降低门控或删除测试；
8. 完成后移动到 archive，更新下一个“Ready”项，不自动 push。

### 必须暂停并请求用户的情况

- 真实桌面点击、设备操作、外部账户、密钥或付费 API；
- 数据删除、不可逆迁移、发布、push、PR、远端写入；
- 产品语义存在多种合理选择且会改变架构或用户体验；
- 需要读取用户 Memory 原文或其他敏感内容；
- 目标范围将从清单内修复扩张为新产品。

### 可以自主继续的情况

- 仓库内只读调查；
- 已批准 story 内的代码、测试、文档和可逆本地迁移夹具；
- 聚焦验证、格式化和本地提交；
- 明确失败后的诊断、最小修复和重新验证。

## 6. 优先级与调度规则

按以下顺序选择工作，而不是只看文件排列：

1. `sev-P0` 数据完整性、安全、authority 或真实验收阻断；
2. 当前批次退出条件所需的 `sev-P1`；
3. 会让全量 gate 不可信的 flaky/MSRV/测试基础债；
4. 已有明确契约和用户旅程的产品能力；
5. 降低未来变更风险的结构治理；
6. `sev-P3` 清理和可选外部 E2E。

同等级按“解锁后续项数量、失败影响范围、可独立提交程度”排序。

## 7. 每切片 Definition of Ready / Done

Ready：

- owner、范围、非目标、依赖和风险明确；
- 现有行为有证据，接口/数据迁移有决策；
- 验收场景可执行，真实环境需求已识别；
- 与活动锁不冲突。

Done：

- 正常、拒绝、边界、并发、重启/恢复按风险覆盖；
- 用户可见改动有真实 smoke；
- 日志无 payload/secret；
- 聚焦测试和阶段 gate 通过；
- 四件套日志自包含；
- TODO、架构状态和用户文档同步；
- 单 concern 提交已创建且未 push。

## 8. 项目级风险登记

| 风险 | 后果 | 控制 |
|---|---|---|
| workspace identity 跨平台不一致 | 误判 store 不匹配或写入另一 authority | B2 canonical identity + identity-only migration |
| 把 G2D 自动化当真机 | 关键 GUI/重启缺陷漏检 | B8 保留 G2D+ 用户观察证据 |
| Evolution 直接按旧设计恢复 | 在已变更 Memory 架构上重建无效功能 | E0–E7 纵向闭环计划与 quality gate |
| GMH 与结构治理双轨修改 | 重复抽象、冲突、超大提交 | 行为契约先于结构重构 |
| flaky test 被当作环境噪声 | 发布门控失真 | B2 消除或隔离并给出根因 |
| 目标模式越权使用密钥/外部系统 | 数据或费用风险 | 明确人工暂停点 |
| “完成所有 TODO”诱发范围膨胀 | 永不收口 | 完成定义 + 新需求先分类 |

## 9. 推荐的目标文本

> 按 `docs/dev/autodream-laputa-product-closure/` 和
> `docs/architecture/todolist-master-execution-plan.md` 持续完成根
> `TODOLIST.md` 的全部活跃事项。先完成 AutoDream evidence 到 typed Memory/Recall
> 的 E0–E7 产品纵向闭环与自动化门，再执行 G2D+ 最终真实桌面验收。严格遵守
> LOCK、单切片提交、四件套日志和不推送规则；不得降低测试门、恢复 Mentle、静默
> fallback、让 AutoDream 自批准或直接写 authority。只有真实桌面、密钥、外部写入、
> 不可逆操作或新产品决策才暂停，其余仓库内工作自主推进。
