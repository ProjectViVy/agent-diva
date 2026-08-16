# 发布说明

需要重启 gateway / CLI / 桌面端以加载新的安全检查。无配置迁移，无数据改写。

CLI `config show` 仍会打码 API key；GUI 控制台日志仍会打码 `token` / `authorization`
字段。这两处是运维密钥防护，不是对话/工具结果里的格式脱敏。

未 push。回滚：还原本片提交即可。
