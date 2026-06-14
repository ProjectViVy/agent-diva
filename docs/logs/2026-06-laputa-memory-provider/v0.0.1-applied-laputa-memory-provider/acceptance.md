# Acceptance

## User-Facing Checks

- With applied Laputa section files present under `.laputa/sections`, build an agent prompt and confirm it includes `## Applied Laputa Authority`.
- Create or leave pending proposals in `.laputa/proposals` and confirm their proposed content is not rendered as default authority.
- Add legacy `SOUL.md`, `IDENTITY.md`, or `USER.md` content and confirm default prompt/subagent authority does not render those files.
- Force a Laputa section read failure and confirm prompt construction still succeeds with degraded memory status and no authority mutation.

## Status

All Story 5.1 acceptance criteria are covered by implementation and targeted tests, subject to the unrelated manager crate validation blocker recorded in `TODOLIST.md`.
