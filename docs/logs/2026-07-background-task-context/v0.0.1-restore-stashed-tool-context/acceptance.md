# Acceptance

1. 执行 `cargo check -p agent-diva-agent`，无 `BackgroundTaskContext` 或 `with_context` 编译错误。
2. 执行 `cargo test -p agent-diva-tools --lib`，背景任务工具及其上下文继承测试通过。
