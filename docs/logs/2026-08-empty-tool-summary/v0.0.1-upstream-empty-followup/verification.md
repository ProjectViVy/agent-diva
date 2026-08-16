# Verification

## Commands

- `cargo test -p agent-diva-agent --lib classify_`
- `cargo test -p agent-diva-agent --lib request_summary_followup`
- `cargo test -p agent-diva-agent --lib grants_and_consumes`
- `cargo test -p agent-diva-agent --lib empty_text_after_tools`
- `cargo fmt -p agent-diva-agent`
- `cargo clippy -p agent-diva-agent --all-targets -- -D warnings`
- `cargo test -p agent-diva-agent --lib`

## Results

- Classifier 5/5：空正文忽略、输入压力（usage / assembly）、输出截断（`length` / completion cap）、空 stop、压力优先于截断。
- `request_summary_followup` 在未耗尽 `max_iterations` 时强制下一拍 summary-only，且每轮一次。
- 既有 `grants_and_consumes_exactly_one_summary_only_pass` 仍绿。
- 集成：工具调用 → `stop`+空正文 → 第三次 `ToolChoiceMode::Disabled` + `max_tokens=8192` + 无工具定义，返回 `recovered summary`，不再走中文机械兜底。
- rustfmt 已应用；`just fmt-check` 通过。
- `cargo clippy -p agent-diva-agent --all-targets -- -D warnings` 通过。
- `just check` 通过。
- `cargo test -p agent-diva-agent --lib` 404/404 通过。
- `just run --help` 列出 CLI 子命令。

## Deferred

- 全量 `just test` / `just ci` 未在本切片跑（改动限于 agent loop，未碰 CLI/GUI 契约）。
- 真机长任务需用户在原会话再跑一次原先会空总结的 `exec` 路径。
