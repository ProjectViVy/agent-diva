# Verification

## Focused

- `cargo test -p agent-diva-sandbox`: 105 tests passed.
- `cargo test -p agent-diva-tools shell::tests`: 13 tests passed.
- `cargo test -p agent-diva-agent tool_assembly`: 19 tests passed.
- `cargo test -p agent-diva-manager`: 65 tests passed.
- Focused clippy for Sandbox, Tooling, Tools, Agent, CLI, and Manager passed with warnings denied.

The shell approval test forces `OnRequest`, receives the pending request, resolves it through the coordinator, and verifies the original call resumes and executes the harmless command once. Manager router tests cover pending filtering and 200/404/409/422 decision semantics.

## Workspace gates

- `just fmt-check`: passed.
- `just check`: passed.
- `just test`: passed.

## Smoke boundary

The Axum router contract tests exercise the real HTTP extraction/routing/response layer in process. A full gateway/provider-backed chat smoke was not used because it would require live model credentials; command resumption itself is covered by the real shell subprocess test.
