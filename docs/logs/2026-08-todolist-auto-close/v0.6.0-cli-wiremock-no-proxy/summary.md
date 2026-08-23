# Summary — CLI-WIREMOCK-502-PREEXISTING

- 版本：`v0.6.0-cli-wiremock-no-proxy`
- 日期：2026-08-23
- 类型：loopback 请求绕过系统代理

## 根因

`ApiClient` 使用 `reqwest::Client::new()`，Windows 上会走 `HTTP_PROXY`，把
wiremock 的 `127.0.0.1` 打到代理，返回 502。

## 做了什么

- base_url 为 `127.0.0.1` / `localhost` / `[::1]` 时 `Client::builder().no_proxy()`。
- 远程 gateway URL 仍走系统代理。

## 影响范围

- `agent-diva-cli/src/client.rs`
- `TODOLIST.md`
- 本日志
