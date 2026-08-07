# agent-diva-core

## OVERVIEW

agent-diva-core holds the cross-cutting domain types and services used by every workspace crate. It owns config, session, memory, event bus, cron, heartbeat, security primitives, and shared error/logging types.

## STRUCTURE

Top-level modules are single `.rs` files; multi-file domains live in directories with `mod.rs`.

- Single-file modules: `ask_user`, `attachment`, `audit`, `audit_parse`, `audit_sink`, `error`, `error_category`, `error_context`, `experience`, `logging`, `rate_limiter`, `reasoning`, `workspace_identity`
- Directory modules: `bus/`, `config/`, `cron/`, `evolution/`, `governance/`, `heartbeat/`, `memory/`, `planning/`, `presence/`, `quality/`, `reports/`, `scheduler/`, `security/`, `session/`, `soul/`, `supervised/`, `todo/`, `token_ledger/`, `utils/`

## WHERE TO LOOK

| Concern | Module | Entry |
|---|---|---|
| Config schema, loader, validation | `config` | `config/mod.rs` |
| Session store, manager, search | `session` | `session/mod.rs` |
| Memory CRUD, recall, provider, working | `memory` | `memory/mod.rs` |
| Event bus | `bus` | `bus/mod.rs` |
| Cron service, types | `cron` | `cron/mod.rs` |
| Heartbeat service, types | `heartbeat` | `heartbeat/mod.rs` |
| Security checks, PII, policy, injection | `security` | `security/mod.rs` |
| Plans, approvals, updates, reports | `planning` | `planning/mod.rs` |
| Governance ledger, policy, coordinator | `governance` | `governance/mod.rs` |
| Token budget and ledger | `token_ledger` | `token_ledger/mod.rs` |
| Error types and result | `error` | `error.rs` |
| Logging setup | `logging` | `logging.rs` |
| Quality metrics, context budget | `quality` | `quality/mod.rs` |
| Presence detection and service | `presence` | `presence/mod.rs` |
| Reasoning types | `reasoning` | `reasoning.rs` |
| Supervised execution, reaper, store | `supervised` | `supervised/mod.rs` |
| Todo store and types | `todo` | `todo/mod.rs` |
| Workspace identity | `workspace_identity` | `workspace_identity.rs` |
| File attachments | `attachment` | `attachment.rs` |
| User-ask flow | `ask_user` | `ask_user.rs` |
| Experience tracking | `experience` | `experience.rs` |
| Evolution types | `evolution` | `evolution/mod.rs` |
| Scheduler bridge and sandbox | `scheduler` | `scheduler/mod.rs` |
| Soul model | `soul` | `soul/mod.rs` |
| Report generation and validation | `reports` | `reports/mod.rs` |

## CONVENTIONS

- Keep cross-cutting domain types in this crate; do not duplicate them downstream.
- Promote a module to a directory once it has more than two sub-concerns.
- Re-export common types from `lib.rs` (`Error`, `Result`, `FileAttachment`, `emit`).
- Leave provider/channel/tool-specific types in their own crates.

## ANTI-PATTERNS

- Do not place agent-loop business logic here; that belongs in `agent-diva-agent`.
- Do not add crate-specific features to core; this crate is a universal dependency.
- Do not introduce provider/channel-only config schemas here.

## NOTES

- `audit_parse`, `audit_sink`, `error_category`, `error_context`, `rate_limiter`, and `reports` are public helper modules but secondary; start with `audit`, `error`, or the parent concern.
- `Cargo.toml` uses workspace dependencies; new external crates here affect the whole workspace lockfile.
