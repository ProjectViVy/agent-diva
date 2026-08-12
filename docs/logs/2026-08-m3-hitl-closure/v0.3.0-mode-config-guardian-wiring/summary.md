# Summary

M3 HITL 收尾 S3：把 Guardian 接入生产 shell 路径，并按 GUI 模式注入不同的
`GuardianConfig`，让 S2 的三模式逻辑在生产真正生效。

## 变更

- `agent-diva-sandbox/src/guardian.rs`：
  - 新增 `GuardianConfig::smart()`（known_safe+read_only 开、learning 关、breaker 松）。
  - 新增 `GuardianConfig::for_ask(AskForApproval)`：谨慎→strict()、智能→smart()、
    信任→liberal()、Never→default()。
- `agent-diva-sandbox/src/orchestrator.rs`：
  - 新增 chained builder `with_guardian_and_exec_policy(Arc<GuardianManager>,
    Option<Arc<ExecPolicyManager>>)`。
  - fallback `check_approval` 拆分 `OnRequest|UnlessTrusted`：信任未知→Skip、
    谨慎未知→NeedsApproval（信任默认放行未知，危险仍由 Guardian 询问）。
- `agent-diva-tools/src/shell.rs::with_approval_backend`：`approval_policy != Never` 时，
  用 `coordinator.command_rules()` 构建 `GuardianConfig::for_ask` + `with_rules` reviewer +
  `ExecPolicyManager::from_command_rule_store`，以 `with_guardian_and_exec_policy` 附加；
  `Never` 保持纯 `ToolOrchestrator::new` 路径（逐字节不变）。

## Impact

- 生产路径首次接入 Guardian：智能/谨慎/信任三模式在生产产生区分行为。
- 已知安全判定复用 coordinator 的 `CommandRuleStore`（S1 引入），不新增双规则文件。
- `Never`（bypass）路径不附加 Guardian，行为不变。