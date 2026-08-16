# Persona 首次设置信封修复

- 日期：2026-08-17
- 切片：首次打开「建立 Persona」报 `unknown Laputa API error`
- 分支：`agent-diva-pro`（未 push）
- 版本目录：`docs/logs/2026-08-persona-setup-envelope/v0.0.1-status-ok-envelope/`

## 目标

桌面端首次设置门在拉 `/api/persona/status` 时不再把成功响应当成失败，用户能看到五文件表单并完成初始化。

## 原因

Manager Persona 成功体是 `{ "status": PersonaStatusView }`。`PersonaStatusView.status` 为
`uninitialized` / `ready` / `incomplete`，外层 `status` 因此是对象。

Tauri `parse_laputa_response` 要求信封 `status == "ok"`。对象不等于 `"ok"`，又读不到
顶层 `message`，于是回落成 `unknown Laputa API error`。打开 GUI 就会命中。

现场 `GET http://127.0.0.1:3000/api/persona/status` 已复现该形状。

## 改动

- Manager `/api/persona/*` 成功响应统一为 `{ "status": "ok", <payload> }`；状态类接口 payload 字段为 `persona`，避免与信封键冲突。
- Tauri 解码：HTTP 2xx 视为成功；从 `error.message` 抽真实错误；兼容旧 `{ "status": PersonaStatusView }`。
- 状态 / 初始化 / 修复命令改为提取 `persona`。

## 未做

- 未重启用户正在跑的 gateway / 桌面进程。
- 未做原生 WebView 五文件填写视觉验收（仍挂 `PERSONA-S2-DESKTOP-SMOKE`）。
