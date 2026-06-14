# Acceptance

Manual acceptance steps:

1. Open the Evolution view and switch to Runs.
2. Confirm the view shows either real AutoDream run rows or the clear message `No AutoDream runs are available yet` / backend-unavailable state.
3. Switch to Audit and confirm changelog rows show timestamp, actor, source proposal, target, change type summary, and rollback availability.
4. Confirm stale, already reverted, and unsupported-action records explain why rollback is unavailable.
5. Switch to Policy and confirm it is a configuration/status summary, not a review queue.
6. Confirm the Policy view includes exactly: `Durable personality, memory, SOP, skill, and policy changes require review before they are applied.`
7. Open Settings > Self Evolution and confirm auto-merge is shown as disabled policy, not an enabled v1 control.
