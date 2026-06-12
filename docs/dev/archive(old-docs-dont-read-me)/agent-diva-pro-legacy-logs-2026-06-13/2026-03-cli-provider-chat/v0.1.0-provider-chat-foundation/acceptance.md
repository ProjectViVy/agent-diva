# Acceptance

1. Run `agent-diva provider list --json` and confirm provider entries include registry default model metadata.
2. Run `agent-diva provider status --json` and confirm the active model/provider plus missing-field readiness are reported.
3. Run `agent-diva provider set --provider deepseek --api-key <key>` and confirm `agents.defaults.model` becomes `deepseek-chat`.
4. Run `agent-diva agent --message "hello" --no-markdown --logs --session cli:test` and confirm the new flags are accepted.
5. Run `agent-diva chat` and confirm `/quit`, `/clear`, `/new`, `/stop` are recognized by the lightweight CLI loop.
