# C1c Skills Reload Verification

## 定向验证

- `cargo check -p agent-diva-agent -p agent-diva-manager`：通过。
- `cargo test -p agent-diva-agent c1c_ --lib`：通过，7 tests。
- `cargo test -p agent-diva-agent memory_distill_reload_only_accepts_applied_outcome --lib`：通过。
- `cargo test -p agent-diva-manager upload_skill_zip_reports_no_change_for_identical_directory --lib`：通过。
- `cargo test -p agent-diva-manager runtime_skills_reload_command_is_workspace_scoped --lib`：通过。
- `git diff --check`：通过；仅有 Windows 换行提示，无 whitespace error。

## 工作区门禁

- `just fmt-check`：通过。
- `just check`：通过，workspace Clippy warnings denied。
- `just test`：通过，workspace tests 与 doctests 全绿。
- `just ci`：通过，包含 feature-gate、BML boundary、Laputa clean-break 和全量测试。
- `cargo run -p agent-diva-cli -- --help`：通过，帮助页正常输出并退出 0。

## 覆盖点

- 同一 Builder 内两个 Session 都只在下一次组装时看到新增技能。
- reload 只携带 `SkillsReload` break reason，其他稳定 section 内容保持不变。
- `Applied`、`ProposalCreated`、`Failed` 三类 `memory_distill` 结果分流正确。
- 重复上传完全相同的技能目录被识别为 no-op，不触发 reload。
- Runtime command 携带 canonical workspace ID 和操作 change ID。
