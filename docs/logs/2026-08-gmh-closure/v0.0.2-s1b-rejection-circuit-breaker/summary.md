# Summary

GMH-41 自治预算与熔断的第一步（S1b）：新增模型/提供方拒绝熔断原语，作为死循环
安全阀，区别于沙箱里的审批拒绝熔断（`GuardianRejectionCircuitBreaker`）。

## 变更

- **新增原语** `agent-diva-core/src/security/rejection_circuit.rs`：
  `RejectionCircuitBreaker { tracker: Arc<ActionTracker>, threshold: u32 }`，复用
  `security/rate_limit.rs` 的 `ActionTracker`（滑动窗口）。提供 `record_rejection()` /
  `is_triggered()` / `rejection_count()` / `threshold()` / `reset()`。Clone 共享同一窗口。
- **配置键** `agent-diva-core/src/security/config.rs` 的 `SecurityConfig` 新增
  `rejection_circuit_window_secs`（默认 60）与 `rejection_circuit_threshold`（默认 50），
  默认值极大，正常操作永不触发，纯死循环熔断安全阀而非预算管理。含 `merge` 逻辑。
- **接线** `agent-diva-agent/src/agent_loop.rs`：`AgentLoop` 新增 `rejection_circuit`
  字段，三个构造点从 `runtime_security` 读取配置；新增 `rejection_circuit()` 测试访问器。
- **迭代入口** `agent-diva-agent/src/agent_loop/turn/iteration.rs`：`start_model_stream`
  循环顶部 `is_triggered()` 命中即返回非 `BudgetExceeded` 熔断错误；provider 调用最终失败
  分支 `record_rejection()` 计数并告警。

## Impact

- 死循环 / 拒绝风暴下，循环在触发阈值后拒绝新一次模型迭代，直到窗口滑过。
- 与审批拒绝熔断（sandbox/guardian.rs）语义隔离，互不复用。
- 默认不改变现有行为（阈值 50 巨大）。