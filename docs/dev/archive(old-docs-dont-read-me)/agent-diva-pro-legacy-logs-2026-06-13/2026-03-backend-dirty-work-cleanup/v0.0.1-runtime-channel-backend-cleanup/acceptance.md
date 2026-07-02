# 验收建议

## 主后端脏工作清理

满足以下观察点时，可认为本轮目标基本达成：

1. gateway 仍可正常启动，CLI 显示的本地 API 地址与实际 runtime 端口一致
2. 更新 Slack channel 配置时，缺少 `app_token` 不再被误当成可启动配置
3. channel 启动失败时，日志能明确指出失败 channel，而不是留下模糊的犹豫态行为
4. provider 相关 HTTP 路由仍可从 `/api/providers*` 访问，`/api/events` 继续可用，但已不再混挂在 provider 路由组
5. runtime 代码结构上已能明确区分 bootstrap、任务启动、关闭三段职责

## 本轮不包含

- GUI 首开/恢复/缓存清理优化
- GUI provider 页面回归
- 自动化测试补充
- `litellm` 与 `sanitize` 的共享净化抽象
