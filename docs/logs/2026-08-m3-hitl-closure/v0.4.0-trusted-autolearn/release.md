# Release

本 slice 为信任模式的自动学习能力，无独立发布物。

## 部署方式

随 `agent-diva-sandbox` / `agent-diva-tools` 常规 crate 发布。

## 说明

- 无停机、无迁移。
- `ExecPolicyManager::policy()` 返回类型由 `&Arc<Policy>` 改为 `Arc<Policy>`（内部
  实现用 `RwLock`），属公开 API 签名变更；无外部调用方受影响。
- 学习规则落独立 `execpolicy-guardian.toml`，不覆盖 coordinator 的 execpolicy.toml。