# Verification

- `cargo check -p agent-diva-migration` — 通过。
- `cargo test -p agent-diva-migration` — 7 passed（含
  `mentle_path_and_unknown_format_are_rejected` 拒绝守卫）。
- 删除前 grep 确认：`mod config_migration|memory_migration|session_migration` 无命中；
  目标文件仅被自身测试函数引用。
- workspace 级 `just fmt-check && just check && just test` 待 GMH 收尾整体门禁
  统一执行（本切片为纯删除，无新增代码）。