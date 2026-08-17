# Verification

- 工具装配测试确认五个基础 BML CRUD schema 位于 CORE，`memory_list` 和 ACTMEM 管理工具仍位于 DEFERRED。
- AgentLoop 回归覆盖首次写零落库、规则全文可见后重试成功，以及下一用户 turn 重新预检。
- `cargo test -p agent-diva-agent --lib`：405/405 通过；随后新增的 MEMRULES 失败关闭用例与跨轮写入用例 2/2 通过。
- `cargo test -p agent-diva-tools --lib`：121/121 通过。
- `just fmt-check`、`just check`、`git diff --check`：通过。
- `just test` 首次被正在运行的 `target/debug/agent-diva.exe` 文件锁阻断；隔离 target 重试在并行链接 CLI/GUI 时耗尽磁盘。没有测试断言失败，隔离构建缓存已删除。
- 用户于 2026-08-18 完成实际 BML CRUD 测试并确认行为正常，接受该环境性全量门禁缺口后直接收口。
