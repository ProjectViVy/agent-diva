# Acceptance

1. 新配置默认 Mentle `enabled=true`、`mode=full`。
2. 当前用户配置包含 `tools.builtin.mentle=true`。
3. 重启 `just diva-gate` 后，Gateway 使用 Mentle feature 和 Full runtime 配置。
4. 将 `mentle.enabled=false` 或 `mentle.mode=off` 可显式关闭 Mentle。

