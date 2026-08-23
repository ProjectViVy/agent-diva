# Verification

- `cargo test -p agent-diva-laputa --lib lock`：`stale_lock_file_with_pid_is_recovered` 通过。
- 未跑全仓库负载复现；修复针对确定的 pid 门控错误。
