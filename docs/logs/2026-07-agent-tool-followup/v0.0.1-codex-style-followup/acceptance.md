# Acceptance

1. Restart gateway with the new agent binary.
2. Approve a plan and let the agent implement (including a final shell/exec check).
3. **Expected:** final assistant bubble is a real summary or Chinese tool synthesis; no  
   `I've completed processing but have no response to give.`
4. Plan generate/approve path still shows 计划报告 / 审批 card when demux succeeds.
5. Cancel mid-turn still reports user stop, not a synthetic tool summary.
