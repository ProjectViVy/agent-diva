# 配置与依赖管理

全局持久规则位于 `~/.agent-diva/execpolicy.toml`，格式复用 `PrefixRule` 的 `allow` 决策。不得新增网络依赖或凭据；路径与规则写入失败必须记录审计信息。
