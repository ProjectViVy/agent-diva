# 关闭运行路径上的敏感格式隐藏

- 日期：2026-08-17
- 切片：按用户决策去掉运行时 PII / 敏感格式替换，不再给记录 id 开特例
- 分支：`agent-diva-pro`（未 push）
- 版本目录：`docs/logs/2026-08-memory-id-pii-redaction/v0.0.2-disable-runtime-pii-hiding/`

## 目标

模型看到的工具结果、入站消息、artifact 回读和 AutoDream 摘要不再把电话、证件号、
卡号、邮箱、记录 id 等替换成 `[REDACTED:…]`。

## 原因

上一轮用「保护 BML id」绕开误伤。用户明确不要这条路，要求直接去掉敏感数据隐藏。
隐藏层既挡住记录 id，也会改写用户原文，导致增删改无法完成。

## 改动

- `PiiConfig` 默认 `enabled: false`，`redact_pii` 在默认配置下原样返回。
- `sanitize_tool_output` 不再调用 `redact_pii`（仍剥 ANSI、标注入）。
- `check_security` 不再因 PII 形状返回 `Sanitize`，邮箱等原文进入模型。
- 工具 artifact 与 AutoDream 证据摘要不再二次脱敏。

## 未做

- 未删 `pii.rs` 检测器；显式 `enabled: true` 仍可测。
- 未改 CLI `config show` 的密钥打码、GUI 控制台对 `token` / `authorization` 的日志打码。
- 未重启用户正在跑的 gateway / 桌面进程。
