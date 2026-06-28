# Acceptance

1. Build the workspace so `target/debug/agent-diva.exe` includes the latest agent-loop changes.
2. Prepare a temp config with a writable workspace, structured runtime logs enabled, and a provider base URL that can fail after prompt assembly.
3. Run `agent-diva --config <temp-config> agent --message "<normal>" --session <id>`.
4. Confirm runtime JSONL contains `system_prompt_budget_measured` with non-zero `estimated_tokens`.
5. Confirm the normal case keeps `overflow=false` and emits no false overflow warning.
6. Prepare a second temp config with a tiny `agents.defaults.context_budget_reserve_tokens` value.
7. Run `agent-diva --config <overflow-config> agent --message "<overflow>" --session <id>`.
8. Confirm runtime JSONL reports `overflow=true` for `system_prompt_budget_measured`.
9. Confirm `gateway.log.<date>` contains `System prompt exceeds reserved token budget`.
10. Re-run `cargo test -p agent-diva-agent system_prompt_budget -- --nocapture` and confirm the coexistence regression test stays green.
