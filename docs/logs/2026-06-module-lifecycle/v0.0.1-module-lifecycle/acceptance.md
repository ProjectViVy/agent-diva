# Acceptance

1. Boot the manager runtime and confirm registered modules are started through `ModuleStartup`.
2. Confirm shutdown stops modules in reverse order without leaving cron running.
3. Run `cargo test -p agent-diva-core -p agent-diva-tooling -p agent-diva-manager` successfully.
4. Track the remaining heartbeat-to-presence wiring gap in `TODOLIST.md`.
