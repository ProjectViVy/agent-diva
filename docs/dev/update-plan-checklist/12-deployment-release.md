# 部署与发布方案

## 构建与验证

发布前执行针对性测试、GUI production build、`just fmt-check`、`just check`、`just test`，并记录到 iteration verification。无数据库迁移或配置迁移步骤。

## 发布影响

更新 agent、manager、CLI/TUI 与 GUI 二进制即可。混合版本仍使用相同 SSE 事件名；新后端输出 snake_case，新 GUI 同时兼容旧状态拼写。

## 回滚

回滚聚焦提交即可恢复旧事件顺序和显示。由于没有持久化 schema 变化，不需要数据回滚。

## 监控

观察 `update_plan` 后是否持续收到 `FinalResponse`，GUI 是否清除输入中状态，以及是否出现重复 checklist 卡片。异常通过既有 tool/event tracing 定位。
