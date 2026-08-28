# agent-diva-e2e

## OVERVIEW

Real-LLM end-to-end test harness. Discovers YAML scenarios, builds an `AgentLoop`, runs multi-turn conversations, and evaluates assertions against responses, tool calls, and file artifacts.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| Scenario runner | `src/runner.rs` |
| Assertion engine | `src/assertions.rs` |
| E2E config / env gating | `src/config.rs` |
| Event collector | `src/collector.rs` |
| Tracer | `src/tracer.rs` |
| Report / types | `src/report.rs`, `src/types.rs` |
| Scenario files | `scripts/e2e/scenarios/` |

## CONVENTIONS

- Tests are skipped unless `DEEPSEEK_API_KEY` or `E2E_API_KEY` is set.
- A scenario is a YAML file describing messages, expected assertions, and tool/file checks.
- Real-LLM tests are intentionally not part of `just ci`; run with `just e2e-test`.

## ANTI-PATTERNS

- Do not add unit tests for provider/client internals here; use the provider crate tests.
- Do not commit recorded responses that contain API keys or private data.

## NOTES

- `just e2e-test` runs `cargo test -p agent-diva-e2e -- --nocapture`.
