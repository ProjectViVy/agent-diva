# 错误处理与边界条件

## 1. 稳定错误域

错误按 `input / provider / candidate / governance / authority / recovery / budget` 分类，Manager、SSE、Tauri、GUI 传递同一 reason code。用户可见消息不包含 prompt、Memory 原文、工具输出或密钥。

## 2. 必须处理的场景

| 场景 | 行为 |
|---|---|
| 没有可验证 evidence | run 完成但 `no_candidates`，不报伪成功 |
| provider 未配置 | `provider_unavailable`，提供设置入口 |
| provider 超时/限流 | 有界重试后 degraded；不产生半提案 |
| 返回非法 schema | 保存脱敏诊断；候选全部拒绝 |
| 候选与现有 Memory 冲突 | 标记 `needs_review`，禁止自动低风险分类 |
| proposal 发布一半崩溃 | 按 deterministic key 恢复剩余项 |
| receipt 过期/撤销 | apply 拒绝，不自动重新审批 |
| typed store 损坏 | AutoDream 可完成分析但禁止发布/apply，并明确 degraded |
| 用户编辑候选 | 旧 digest/receipt 失效 |
| 用户拒绝 | suppression 生效，不重复骚扰 |
| rollback | record tombstone/撤销、proposal/changelog/audit 同步 |

## 3. 降级原则

只有“生成候选”可降级为空结果；治理或 authority 失败必须硬失败。Recall 不得退回 legacy。AutoDream 不得因 provider 错误执行 shell、自行修配置或扩大权限。
