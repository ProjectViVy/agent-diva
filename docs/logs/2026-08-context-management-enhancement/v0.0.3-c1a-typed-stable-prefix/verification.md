# C1a Verification

## 定向验证

- `cargo test -p agent-diva-providers --lib --no-fail-fast`：122 passed。
- `cargo test -p agent-diva-agent --lib --no-fail-fast`：396 passed。
- `cargo test -p agent-diva-agent --test compaction_integration --no-fail-fast`：11 passed。
- `cargo test -p agent-diva-agent --test compaction_e2e --no-fail-fast`：15 passed。
- `cargo clippy -p agent-diva-providers --lib -- -D warnings`：通过。
- `cargo clippy -p agent-diva-agent --lib -- -D warnings`：通过。

## 工作区门禁

- `just fmt-check`：通过。
- `just check`：通过。
- `just test`：通过（`cargo test --all` 全绿）。

## 核心断言

- T1：仅 wall-clock 改变时 stable system 相同。
- T2：Working Memory 改变不影响 stable system。
- T3：Recall 空/非空不影响 stable system。
- Anthropic wire conversion 保持 stable system 独立，runtime envelope 在 history 后且
  current user 内容保持最后。
- reactive compaction 前后复用完全相同的 runtime context envelope。
- 代码扫描确认目标装配路径无 `insert(1)`，且本切片未修改 `apply_cache_control`。
