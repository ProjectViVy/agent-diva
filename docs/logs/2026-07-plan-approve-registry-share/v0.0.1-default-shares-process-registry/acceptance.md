# Acceptance

1. Open agent-diva-gui, select **Plan** mode, send a planning request.
2. Wait until the plan approval card appears with a complete Markdown plan.
3. Click **Execute / 执行** (any context policy: retain / compact / clear).
4. **Expected:** no `plan draft not found for session` error; system message confirms approval; agent begins implementing the plan in Agent mode.
5. **Reject if:** approve still returns draft-not-found, or execution never starts after a successful approve.
