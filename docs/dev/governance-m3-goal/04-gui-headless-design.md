# GUI 决策中心与 CLI/headless 设计

## GUI 信息架构

采用“全局抽屉 + 就地卡片”：

- App shell 显示全局 pending badge；点击打开审批抽屉；
- 抽屉按风险/到期时间排序，可按 domain/status/source session 筛选；
- Plan、Command、Evolution 原位置保留就地卡片，均消费同一 server projection；
- mutation in-flight、transport error 和 authoritative terminal fact 分离；
- 本地 cache 只加速，不能覆盖更高 version 的 server state。

详情展示对象 ID/version、风险、capability、scope、TTL、证据 metadata、安全 diff/摘要、
grant 解释、来源会话和 typed reason。Command 原文仅在活跃 waiter 中展示；日志、fixture、
ledger 和重启恢复均不得复制它。

## GUI 行为

- stale version：禁用旧卡动作，提示刷新并拉取最新详情；
- committed-but-refresh-failed：显示“动作可能已提交”，只刷新，不自动重发 decision；
- reconnect：cursor 增量恢复并按 event ID/version 去重；
- stop/cancel：立即进入 submitting，服务端 Revoked 后才显示终态；
- expired：服务端状态为权威，UI countdown 只作提示；
- Memory edit-and-approve：先保存新 proposal revision并观察旧 request revoked、新 request
  Pending，再允许提交 decision；
- 高风险缺证据：后端拒绝，UI 同时禁用并解释；
- 可访问性：按钮/状态有文字与键盘焦点，目标至少 44px，不只用颜色表达。

## CLI 交互模式

- 显示 domain、风险、安全摘要、scope、到期时间和允许的 grant；
- 用户明确选择 allow once/session/rule 或 deny/cancel；不接受空输入为 allow；
- terminal 输出包含 request ID、version、status、reason code；
- Ctrl+C/会话 stop 写 Revoked，不遗留可执行 Allowed。

## Headless 默认与 queue

- 非交互默认立即返回 `approval_required_noninteractive`，退出码固定为非零，不执行；
- 只有显式 approval mode `queue` 才尝试 durable queue；
- queue 要求 Manager 可达且 domain payload 可持久恢复；Command raw payload 不持久化，
  因而 direct headless Command 默认不可 queue；
- Plan/Memory queue 成功返回 request ID、status URL/命令、expires_at，不无限等待；
- Manager unavailable、queue 不支持或 payload 不可恢复返回 `approval_queue_unavailable`；
- JSON 输出与人类输出语义一致，禁止把 pending 当 success。

## 隔离桌面约束

使用全新隔离 profile，不读取原 profile/key；通过本地 fixture、测试 endpoint 或确定性 fake
生成三域 Pending。不得为方便 smoke 临时增加生产后门；fixture 注入口仅存在于测试构建或
测试 harness。debug 外部 gateway 与 release embedded gateway 都要验证，不调用 provider。
