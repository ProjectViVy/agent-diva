# Acceptance

1. Open GUI settings and verify the General page shows doctor health, provider/channel readiness counts, and resolved paths.
2. Open Provider settings and verify current provider/model plus readiness details are visible.
3. Open Channel settings and verify readiness and missing fields are visible for the selected channel.
4. Run `agent-diva agent --config <file> --workspace <dir> --message "hello"` and confirm the command completes successfully.
5. Run `agent-diva agent --config <file> --no-markdown --logs --session cli:test --message "hello"` and confirm the command completes successfully.
6. Run `agent-diva chat --help` and confirm the lightweight chat flags are listed.
