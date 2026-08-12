# Summary

M3 HITL 收尾 S4：实现信任模式自动学习（Guardian `create_rule` 落盘）。

## 变更

- `agent-diva-sandbox/src/command_rules.rs`：新增 `CommandRuleStore::path()` 访问器，
  供 shell 接线推导 Guardian 学习文件路径。
- `agent-diva-sandbox/src/exec_policy.rs`：
  - `policy` 字段由 `Arc<Policy>` 改为 `parking_lot::RwLock<Arc<Policy>>`，使共享
    `Arc<ExecPolicyManager>` 可经 `&self` 原子换策略（`policy()` 改返回 `Arc<Policy>`）。
  - 新增 `append_amendment_shared(&self, amendment)`（幂等：校验/去重/落盘/更新内存），
    `append_amendment(&mut self)` 委托之。
  - 新增 `with_rules_path(path)` 设置持久化路径。
- `agent-diva-sandbox/src/orchestrator.rs::preflight_guardian`：`create_rule` 分支从
  空 stub 改为调用 `exec_policy.append_amendment_shared`，把信任模式放行的未知命令
  自动学习为 Allow 规则。
- `agent-diva-tools/src/shell.rs`：Guardian 的 `ExecPolicyManager` 设置持久化路径为
  coordinator 规则文件同目录下的 `execpolicy-guardian.toml`（**独立文件**，避免覆盖
  `CommandRuleStore` 的 execpolicy.toml 格式）。

## Impact

- 信任模式放行未知命令后自动学习为 Allow 规则；进程内后续 known-safe 命中，且落盘
  到独立 guardian 文件。
- 不污染 coordinator 的 execpolicy.toml（双规则存储分歧缓解）。