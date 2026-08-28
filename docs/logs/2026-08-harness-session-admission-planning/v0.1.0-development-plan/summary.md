# HARNESS Session Admission 开发计划

## 结论

`HARNESS-SESSION-ADMISSION-BOUNDED-QUEUE` 已从单行需求扩展为可执行的六阶段计划，
基线为 13 个工程日，计划窗口为 2026-08-31 至 2026-09-16，并预留至 2026-09-18 的
2 个工程日风险缓冲。

本次只完成计划与排期，不修改产品代码。计划以 1 名主开发连续投入估算；若并行 Epic
占用同一运行时文件，应在独立 worktree 开发，并以门禁结果而非日期强行推进。

## 阶段

| 阶段 | 日期 | 工期 | 交付结果 |
| --- | --- | ---: | --- |
| HQ-00 | 08-31 ～ 09-01 | 2d | 合同、默认值候选、并发所有权和停止条件冻结 |
| HQ-01 | 09-02 ～ 09-04 | 3d | Core bounded FIFO / lease / timeout / cancel / eviction |
| HQ-02 | 09-07 ～ 09-09 | 3d | Agent dispatcher、统一 admission seam、跨 session 并行 |
| HQ-03 | 09-10 ～ 09-11 | 2d | Stop/Reset、配置、typed outcome 与可观察投影 |
| HQ-04 | 09-14 ～ 09-15 | 2d | 跨入口测试、故障注入与用户可见背压验证 |
| HQ-05 | 09-16 | 1d | 全门禁、smoke、文档、回滚与关闭判定 |

## 关键判断

当前 `AgentLoop::run(&mut self)` 全局串行处理 inbound，且 turn 使用多个 loop 级可变状态。
因此，单纯新增 per-session mutex 不能实现“同 session 串行、不同 session 并行”。HQ-00
必须先冻结 dispatcher + per-session worker/lease 的所有权模型；若只能通过复制 AgentLoop、
重写 MessageBus 或破坏现有领域边界实现，则停止并重新评审。

## 范围边界

- 包含：per-session FIFO、队列深度、等待超时、取消、idle slot 回收、typed outcome、
  queue 指标与 Manager/CLI/GUI/Channel 现有合同投影。
- 不包含：重写 MessageBus、EventBus Hook、小时/天 token queue、A2A、Neuro-Link、
  BML/Persona/Evolution 写路径或任意外部插件执行。
- 深度定义：`max_queue_depth` 只计算等待请求，不含当前 running turn；最终默认值由 HQ-00
  表征和配置兼容性评审冻结。

## 影响范围预估

主要候选路径为 `agent-diva-core/src/session/`、
`agent-diva-agent/src/agent_loop.rs`、`agent-diva-agent/src/agent_loop/turn/admission.rs`、
`agent-diva-agent/src/agent_loop/loop_runtime_control.rs`、Manager runtime/chat 接口，以及对应
CLI/GUI/Channel 投影与测试。实际施工必须按阶段进一步收窄 LOCK scope。
