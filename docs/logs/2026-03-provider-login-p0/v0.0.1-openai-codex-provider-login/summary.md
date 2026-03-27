# Summary

- 实现了 `openai-codex` 的 provider metadata、配置外 auth store、CLI 登录闭环和专用 runtime backend。
- `agent-diva provider login/status/logout/use/refresh` 已接入统一 auth service。
- `openai-codex` 不再依赖 `config.json` 存储凭据，OAuth token 改为落在独立 auth store。
- GUI provider 设置页已接入 `openai-codex` 的认证状态展示、浏览器登录、redirect 回填、profile 切换、refresh 和 logout。
