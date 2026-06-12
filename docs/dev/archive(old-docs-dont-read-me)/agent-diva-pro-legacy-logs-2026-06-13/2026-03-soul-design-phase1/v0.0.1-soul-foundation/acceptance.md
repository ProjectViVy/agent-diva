# Acceptance

## User-Facing Acceptance Steps
1. Run `agent-diva onboard` and verify workspace contains:
   - `SOUL.md`
   - `IDENTITY.md`
   - `USER.md`
   - `BOOTSTRAP.md`
2. Start agent (`gateway`, `agent`, or `tui`) and verify no regression on basic conversation.
3. Edit `SOUL.md`/`IDENTITY.md`/`USER.md` through file tools in a conversation and verify final response includes a transparency notice.
4. Confirm subagent tasks still execute and carry inherited identity summary.
5. Mark bootstrap complete by editing `BOOTSTRAP.md` through tools, then verify `workspace/.agent-diva/soul-state.json` records completion and bootstrap content stops being injected in subsequent rounds.

## Acceptance Result
- Implementation complete and local validation passed.
