# 可行性评估

## 1. 总评

方案技术可行，推荐度高于 clean-break 或直接回迁 deep 实现。当前代码已经存在可利用的 seam：AgentLoop 子模块、Manager domain handlers/runtime 子模块、Tauri `AgentState` 代理、GUI API 文件和大量局部测试。主要风险是行为回归和并行改动冲突，不是技术栈缺失。

## 2. 方案比较

| 方案 | 产品能力保持 | 风险 | 工期 | 裁决 |
| --- | --- | --- | --- | --- |
| A. 原位分阶段治理 | 高 | 中 | 中 | 推荐 |
| B. 回迁 deep crates/API | 低 | 极高 | 高 | 拒绝 |
| C. 仅机械拆文件 | 中 | 中 | 低 | 不足 |
| D. 保持现状只补功能 | 低 | 持续升高 | 表面低 | 拒绝 |

## 3. 影响矩阵

| 能力 | 风险 | 主要原因 | 缓解 |
| --- | --- | --- | --- |
| 普通 chat/tool loop | 高 | async/stream/tool 顺序敏感 | characterization + event trace |
| Plan approval/execution | 高 | phase、registry、CAS 联动 | phase matrix + stale revision tests |
| stop/cancel | 高 | late event/side effect | cooperative cancel race tests |
| session/history | 中 | cache 与持久化投影 | restart + reload tests |
| Manager API | 中 | 路由/DTO 兼容 | route snapshot + client tests |
| GUI startup | 中 | debug/release gateway 差异 | 两种模式 smoke |
| Pet/Host LOCAL | 低到中 | 拆 command 时误迁业务 | capability ledger |
| Laputa/AutoDream | 中 | 代理链长 | 先搬文件，后改语义 |

## 4. 关键技术风险

### Rust borrow 与 async 边界

当前 `process_inbound_message_inner` 长时间借用 `&mut self`。直接拆成多个持有 `&mut AgentLoop` 的异步对象容易制造 borrow 冲突。第一阶段应使用普通关联函数和短生命周期参数，只有边界稳定后才引入独立 coordinator struct。

### 事件顺序

Manager 将 `AgentBusEvent` 映射为 SSE，入口见 [handlers.rs](../../../agent-diva-manager/src/handlers.rs:431)；GUI 又把流事件折叠为本地 message/plan/tool 状态。移动 event emission 可能造成 UI 重复、迟到或空 placeholder。必须把顺序当作兼容契约测试。

### 双权威

deep 的“server-owned projection”是正确原则，但当前 GUI 已有 session cache、plan in-flight 和 stream placeholder。治理不能粗暴删除这些 UX 状态；应区分：

- domain fact：仅 server；
- optimistic/in-flight：可 local，但必须可被 server projection 覆盖；
- cache：只加速，不作为审批或执行事实。

### 共享 DTO

Rust/TypeScript 一次性 codegen 会新增构建链和版本耦合。先用 fixture contract tests 能以更低风险识别漂移，再决定是否引入生成器。

## 5. 依赖风险

Phase 0–4 不需要新增第三方依赖。Phase 5 的代码生成属于独立 ADR，不得夹带进入结构重构。这样可避免“为治理新增另一套基础设施”。

## 6. 成功条件

- 每个切片都能保持既有 API、schema 和真实用户旅程；
- 不需要同时维护 legacy/new runtime；
- 任何迁移中的 capability 都有明确状态，不能静默返回空成功；
- 回滚只需撤销当前切片，不需要数据恢复。
