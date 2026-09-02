# Verification

| 命令/检查 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `just fmt-check` | 通过 |
| `just check` | 通过（workspace clippy，`-D warnings`） |
| `just test` | 通过；workspace 单元、集成、TCK 与 doc-tests 全部通过（已有 ignored/perf 测试保持 ignored） |
| `cargo check --manifest-path agent-diva-gui/src-tauri/Cargo.toml` | 通过 |
| `pnpm test -- --run`（`agent-diva-gui`） | 通过，76 个 test files / 536 tests |
| `pnpm build`（`agent-diva-gui`） | 通过；仅有 Vite 大 chunk warning，无构建错误 |
| `git diff --check` | 通过；仅报告仓库既有 CRLF 转换提示 |
| `pnpm dev -- --host 127.0.0.1` | Vite 可启动并监听 `http://localhost:1420/`，随后已停止进程 |
| agent-browser / Playwright 页面冒烟 | 未执行：当前环境没有 `agent-browser` 可执行文件、可用 Playwright 浏览器包或 Edge/Chrome/Firefox 二进制；不宣称真实桌面通过 |

新增/加强覆盖：

- owner context wire/schema、AgentLoop metadata 和 pacing context 透传；
- semantic Presentation 的 thinking → speaking/tool → waiting/final/error 生命周期与持久化顺序；
- typed client 的 hello/session/replay/ACK、重复序号、state gap resume、协议不匹配和 cursor 隔离；
- reducer 的 replay 不触发 TTS、live subtitle 触发 TTS、tool lifecycle 与 clear；
- GUI 既有 Mate/desktop overlay 测试迁移到 `neuro-link-presentation` 事件。

真实验收仍需在可启动 Tauri/桌面浏览器的工作站执行：发送普通消息、排队取消、计划批准继续、
WS 断开后重连/回放，以及 Mate 字幕/TTS/表情变化。
