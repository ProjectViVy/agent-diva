# Verification

- `cargo test -p agent-diva-core mentle_config --lib`：通过，2 tests。
- `cargo test -p agent-diva-agent tool_config::mentle --lib`：通过，3 tests。
- `cargo test -p agent-diva-agent --features mentle mentle --lib`：26 passed，2 个 Mentle runtime prompt 断言失败。
- CLI 可执行文件验证受现有 `agent-diva.exe` Gateway 进程占用影响，未结束该用户进程。
- 当前配置文件通过 JSON 解析并已设置 `mentle.enabled=true`、`mentle.mode=full`、`tools.builtin.mentle=true`。

