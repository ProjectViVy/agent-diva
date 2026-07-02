# v0.0.7 Acceptance

## Acceptance Checks

1. The root workspace no longer lists `agent-diva-nano` as a member.
2. `agent-diva-cli` no longer defines a local `nano` feature path.
3. `agent-diva-cli` no longer uses `agent_diva_nano` in code.
4. The main CLI `gateway run` path is manager-backed only.
5. The `external/agent-diva-nano/` directory still exists on disk and is not deleted in this round.
6. Main-repo validation no longer includes any nano-local cargo commands.
