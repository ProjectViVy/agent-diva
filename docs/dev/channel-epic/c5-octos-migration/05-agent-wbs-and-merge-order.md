# Agent WBS, ownership, and merge order

## Gate 0 — documentation freeze

The Lead owns this package, v0.1.8 logs, TODOLIST, and locks. Before product edits, a read-only
reviewer verifies the Octos pin, matrix completeness, shared ADRs, platform specs, and lack of open
decision requests. Commit the freeze separately from implementation.

## Gate 1 — shared implementation

Only the Lead edits:

- `agent-diva-channels/src/adapter.rs`
- `agent-diva-channels/src/adapters/mod.rs` and shared support modules
- `agent-diva-channels/src/lib.rs`
- `agent-diva-channels/Cargo.toml`
- shared capability evidence/TCK harness

Deliver `AdapterServices`, `ChannelAttachmentStore`, access-policy helper, envelope/receipt/error
builders, test endpoint/fake-service conventions, and `build_active_adapters`. Run focused compile,
unit tests, clippy, and MSRV before creating worker branches. Commit as one shared-contract concern.

## Gate 2 — platform worktrees

Create three branches from the exact Gate 1 commit:

| Worker | Owned product files | Owned tests/fixtures | Forbidden shared files |
| --- | --- | --- | --- |
| A | `adapters/telegram.rs`, `adapters/discord.rs` | only Telegram/Discord fixtures and tests | adapter contract, mod/lib, Cargo, global manifest |
| B | `adapters/feishu.rs`, `adapters/dingtalk.rs` | only Feishu/DingTalk fixtures and tests | same |
| C | `adapters/email.rs`, `adapters/qq.rs` | only Email/QQ fixtures, tests, live QQ harness | same |

The Lead pre-registers these non-overlapping scopes in root `LOCK.md` with branch/worktree, owner,
heartbeat, and expiry before workers write. Each worker reads both repository rules and this package,
commits one platform at a time, and does not merge or push.

If a worker needs a shared change, it records `decision-requests.md` and stops at that seam. The Lead
resolves the request in the main isolated worktree, updates ADR/matrix/evidence as needed, validates,
commits, and tells affected workers how to rebase/cherry-pick. Workers never race on shared files.

## Gate 3 — fixed integration order

Integrate focused commits into `feat/channel-epic` in this order:

1. shared adapter/services contract;
2. Telegram;
3. Discord;
4. Feishu;
5. DingTalk;
6. Email;
7. QQ;
8. global capability manifest/TCK and live harness;
9. C5-I/V documentation and TODOLIST closeout.

After each platform, run its tests, `cargo clippy -p agent-diva-channels --all-targets -- -D warnings`,
and `git diff --check`. Reject a commit that changes an unowned platform or legacy production path.

## Gate 4 — cross-review

After workers finish, reassign them read-only:

- A reviews Feishu/DingTalk for no-op capability or ACK/dedup errors.
- B reviews Email/QQ for blocking, resume, attachment, and receipt errors.
- C reviews Telegram/Discord for thread, rate-limit, and lifecycle errors.
- Lead audits shared contracts, evidence completeness, public API docs, and C6 compatibility.

Findings are fixed by the owning worker in a new focused commit. Reviewers do not patch another
worker's files.

## Commit and handoff contract

Every completed concern is committed automatically with an English Conventional Commit subject and
a validation note. Before commit, inspect all untracked files, stage explicit owned paths, and exclude
unrelated changes. Do not push. Release or hand off every lock with commit IDs, tests, known limits,
and the next exact action.
