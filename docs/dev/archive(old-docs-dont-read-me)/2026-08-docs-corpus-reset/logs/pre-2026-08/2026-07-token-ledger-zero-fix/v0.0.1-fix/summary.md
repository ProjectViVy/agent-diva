# Token Ledger Missing Usage Fix

## What was changed
Fixed a bug in `agent-diva-providers/src/openai_compatible.rs` where the streaming endpoint parser ignored the `usage` block if the stream chunk also contained `choices`. 
Some providers (like DeepSeek) append the `usage` JSON payload inside the final chunk alongside `choices` (which contains `finish_reason: "stop"`). Because the code unconditionally continued the loop if `choices` were present without first assigning the parsed `usage` variable, the usage metrics were entirely dropped. This resulted in `0` token usage being recorded in the session, and since `total_tokens == 0` causes the `agent_loop` to skip appending the record, the token ledger never updated.

## Impact range
- `agent-diva-providers/src/openai_compatible.rs`: Stream chunk parsing logic.
