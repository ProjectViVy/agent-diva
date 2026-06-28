# Verification

- Ran `cargo fmt --all`
- Ran `cargo test -p agent-diva-agent context_budget -- --nocapture`
- Ran `cargo test -p agent-diva-agent system_prompt_budget -- --nocapture`
- Ran `cargo check -p agent-diva-agent`

- Manual QA:
- `target/debug/agent-diva.exe --config <temp-config> agent --message "normal budget measurement" --session g004-c001`
  - Observed `system_prompt_budget_measured` in runtime JSONL with `measured_tokens=1072`, `reserve_tokens=4000`, `overflow=false`, and no false overflow warning before the provider connection failed.
- `target/debug/agent-diva.exe --config <temp-config> agent --message "overflow budget measurement" --session g004-c002`
  - Observed `system_prompt_budget_measured` in runtime JSONL with `measured_tokens=1072`, `reserve_tokens=8`, `overflow=true`, plus a `System prompt exceeds reserved token budget` warning in `gateway.log.<date>` before the provider connection failed.

- Evidence artifacts:
- `.omo/ulw-loop/evidence/wave1-e10-s1-c001-system-prompt-measurement.txt`
- `.omo/ulw-loop/evidence/wave1-e10-s1-c002-system-prompt-overflow-warning.txt`
- `.omo/ulw-loop/evidence/wave1-e10-s1-c003-request-budget-regression.txt`
