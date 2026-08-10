# C1b Verification

## 定向验证

- `cargo test -p agent-diva-tooling --lib --no-fail-fast`：31 passed。
- `cargo test -p agent-diva-agent --lib c1b --no-fail-fast`：1 passed。
- `cargo test -p agent-diva-agent --lib custom_tools_follow_the_sorted_core_prefix --no-fail-fast`：1 passed。
- `cargo test -p agent-diva-tools --lib --no-fail-fast`：115 passed。
- `cargo test -p agent-diva-agent --lib --no-fail-fast`：397 passed。
- `cargo clippy -p agent-diva-tooling --lib -- -D warnings`：通过。
- `cargo clippy -p agent-diva-tools --lib -- -D warnings`：通过。
- `cargo clippy -p agent-diva-agent --lib -- -D warnings`：通过。

## 工作区门禁

- `just fmt-check`：通过。
- `just check`：通过。
- `just test`：通过（`cargo test --all` 全绿）。首轮调用因外层 120 秒工具超时关闭
  输出管道而出现 `BrokenPipe`，延长到 300 秒后原命令完整重跑并以 exit 0 结束。
- `cargo run -p agent-diva-cli -- --help`：通过，帮助页正常列出命令并退出 0。

## 核心断言

- CORE definitions 构成连续前缀，DEFERRED/MCP/custom definitions 构成后缀。
- 两个分区各自按工具名字典序输出。
- 相反注册顺序、重复 `get_definitions()` 与独立 ToolAssembly 重建的完整 JSON 字节一致。
- schema object 键递归规范化；`required` 等 array 顺序保持不变。
- 代码差异不包含 `agent-diva-providers` 或 `apply_cache_control`。
