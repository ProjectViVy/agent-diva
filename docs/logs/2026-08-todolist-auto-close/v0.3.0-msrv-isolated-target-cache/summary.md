# Summary — MSRV-ISOLATED-TARGET-CACHE

- 版本：`v0.3.0-msrv-isolated-target-cache`
- 日期：2026-08-23
- 类型：验证命令固化

## 背景

所有 `cargo +1.80` 探测必须使用独立 `CARGO_TARGET_DIR`，避免污染默认 `target/`。
此前只写在 TODOLIST，没有配方。

## 做了什么

- `justfile` 增加 `msrv-probe`（Windows / Unix 各一份），强制
  `CARGO_TARGET_DIR=target/msrv-1.80`（落在已 gitignore 的 `target/` 下，且不是默认
  `target/debug`）。
- 未跑 1.80 全量 `check`/`test`；那是 `WORKSPACE-MSRS-1.80-DEPENDENCY-CONFLICTS`。
- 未改 `just check` / `just ci` / `just test`。

## 影响范围

- `justfile`
- `TODOLIST.md`
- 本日志
