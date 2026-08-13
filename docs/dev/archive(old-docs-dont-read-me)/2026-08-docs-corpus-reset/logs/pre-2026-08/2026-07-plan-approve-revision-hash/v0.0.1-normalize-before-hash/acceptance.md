# Acceptance

1. Restart gateway and rebuild/restart agent-diva-gui (desktop binary must include Tauri command change).
2. Plan mode: generate a plan until the approval card appears.
3. Click **批准并开始执行** with any context policy.
4. **Expected:** no `plan report revision conflict`; approval system message; agent starts executing.
5. **Reject if:** conflict error returns, or approve succeeds but execution never starts.
