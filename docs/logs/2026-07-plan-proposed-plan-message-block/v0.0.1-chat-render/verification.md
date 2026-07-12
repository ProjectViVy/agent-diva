# Verification

```text
cd agent-diva-gui
npx vitest run src/components/planning/proposedPlanMessage.test.ts src/components/planning/AgentMessageBody.test.ts
npx vue-tsc --noEmit
```

| Check | Result |
| --- | --- |
| proposedPlanMessage tests (5) | pass |
| AgentMessageBody tests (2) | pass |
| vue-tsc | pass |

Manual: open a chat whose history still has `<proposed_plan>`; tags should no longer show raw; a blue 计划 block should appear.
