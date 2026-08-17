# 验证记录

## 自动化验证

- `pnpm test`：通过，64 个测试文件、456 个测试用例。
- `pnpm build`：通过；Vite 仅报告项目已有的大 chunk 警告。
- `cargo test -p agent-diva-agent test_add_tool_result`：通过。
- `cargo fmt --all -- --check`：通过。
- `just fmt-check`：通过。
- `just check`：通过，Clippy `-D warnings` 通过。
- `CARGO_TARGET_DIR=C:\tmp\agent-diva-chat-clean-test-target just test`：通过，workspace 测试与 doctest 全量通过。
- `git diff --check`：通过。

## 启动 smoke

- 独立 Vite 进程：`pnpm exec vite --host 127.0.0.1 --port 4173`。
- `GET http://127.0.0.1:4173/` 返回 HTTP 200，入口包含 `#app` 挂载容器。
- 当前环境没有 `agent-browser` 可执行文件，因此未执行浏览器截图/控制台检查；启动进程已在验证后停止。

## 环境说明

- 默认 `just test` 首次尝试因正在运行的 `agent-diva.exe` 锁定 `target\debug\agent-diva.exe`，Cargo 报 Windows `os error 5`；未结束用户进程，改用独立 Cargo target 后全量通过。
