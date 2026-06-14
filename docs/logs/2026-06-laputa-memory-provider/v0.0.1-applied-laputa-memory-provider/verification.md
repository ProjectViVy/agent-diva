# Verification

## Commands

- `cargo test -p agent-diva-laputa memory_provider` - passed.
- `cargo test -p agent-diva-laputa` - passed.
- `cargo test -p agent-diva-agent context` - passed.
- `cargo test -p agent-diva-agent subagent` - passed.
- `cargo check -p agent-diva-agent` - passed.

## Deferred / Blocked Validation

- `cargo check -p agent-diva-manager` was attempted and blocked by a pre-existing unrelated exhaustiveness error in `agent-diva-manager/src/handlers/autodream.rs` for `AutoDreamError::InputCollection(_)` and `AutoDreamError::ProposalPersistence(_)`. Captured in `TODOLIST.md`.
- `cargo fmt --check` was attempted and blocked by pre-existing workspace rustfmt drift outside this story scope. Touched story files were formatted with `rustfmt`.

## Evidence

Targeted tests verify:

- Applied Laputa sections render in the prompt.
- Pending/unapplied proposals are not rendered as authority.
- Legacy `SOUL.md`, `IDENTITY.md`, and `USER.md` content is excluded from default context and subagent prompt authority.
- Laputa read failures degrade without authority writes or proposal creation.
