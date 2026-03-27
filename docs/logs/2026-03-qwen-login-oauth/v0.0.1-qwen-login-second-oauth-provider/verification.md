# Verification

执行与结果：

- `cargo test -p agent-diva-core auth -- --nocapture`
  - 结果：通过
- `cargo test -p agent-diva-providers provider_auth -- --nocapture`
  - 结果：通过
- `cargo fmt --all -- --check`
  - 结果：通过
- `cargo test -p agent-diva-cli --no-run`
  - 结果：通过
- `cargo test -p agent-diva-gui --no-run`
  - 结果：失败，阻塞原因为磁盘空间耗尽，不是 `qwen-login` 代码路径类型错误

补充观察：

- GUI 编译失败时的核心错误为 `No space left on device (os error 28)`，发生在 `target/debug` 与若干 Tauri / objc2 / aws-lc 依赖构建阶段。
- 在清理 `target/debug` 释放空间后，CLI 编译级 smoke 可以通过，说明 `qwen-login` 对 CLI、manager、provider-auth 链路的 Rust 编译影响已收敛。
- 最小真实 CLI smoke `./target/debug/agent-diva provider status qwen-login --json` 未完成，原因同样是环境磁盘已被 GUI 构建重新占满，启动时创建日志目录失败。

说明：

- 本轮实现包含 `agent-diva-gui` 的 Tauri command 改动，但 GUI 编译验证受当前环境容量限制，未能完成最终 `--no-run` 验证。
- 当前 `QwenLoginOAuthBackend` 中的 OAuth endpoint 常量基于 Qwen Code 客户端行为假设与既有计划约束实现；如后续验证发现契约有差异，只需调整 backend 常量与请求参数，不影响当前框架分层。
