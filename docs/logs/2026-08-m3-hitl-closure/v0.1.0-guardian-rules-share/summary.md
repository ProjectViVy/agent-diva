# Summary

M3 HITL 收尾 S1：让 Guardian 复用生产权威规则源（`CommandRuleStore`），并为后续
三模式接线做准备。纯重构，无生产行为变更。

## 变更

- `agent-diva-sandbox/src/orchestrator.rs`：`ToolOrchestrator.exec_policy` 字段由
  `Option<ExecPolicyManager>` 改为 `Option<Arc<ExecPolicyManager>>`，`with_exec_policy`
  与 `with_exec_policy_and_guardian` 构造器参数同步改 `Arc`，供 orchestrator 与
  Guardian reviewer 共享同一实例（`check_approval` 经 Arc 解引用调用不变）。
- `agent-diva-sandbox/src/exec_policy.rs`：新增 `ExecPolicyManager::from_command_rule_store
  (&CommandRuleStore)` — 遍历生产 `CommandRuleStore.list()`，把 enabled+allow 规则转成
  `PrefixRule{pattern, Decision::Allow}`，复用 `with_policy`。让 Guardian 的
  `is_known_safe` 复用与 coordinator 相同的规则源。
- `agent-diva-sandbox/src/command_rules.rs`：新增 `CommandRuleStore::allows_tokens(&[String])`
  精确 token 匹配；`allows(&str)` 改为复用它（避免重复 shell_words::split）。
- `agent-diva-sandbox/src/guardian.rs`：`DefaultGuardianReviewer` 新增 `rules:
  Option<Arc<CommandRuleStore>>` 字段 + `with_rules(approval_policy, rules)` 构造器；
  `is_known_safe` 在 exec_policy 之外也支持 `rules.allows_tokens`。

## Impact

- 无生产行为变更（Guardian 仍未接入生产，仅新增能力与共享路径）。
- 后续 S3 接线时，Guardian reviewer 可直接用 coordinator 的 `CommandRuleStore`
  判定 known-safe，避免双规则存储互相覆盖。完整统一（单 store）超出本次范围，记入
  S3 summary。