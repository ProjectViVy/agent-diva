# Verification

## Testing Method
1. Ran integration and unit tests for the providers via `cargo test -p agent-diva-providers`.
2. Verified the stream parser correctly captures `usage` metadata when the chunk containing it also contains `choices`.

## Results
- The parser now successfully maps and retains `usage` data from stream chunks.
- The `agent_loop` will receive `total_tokens > 0` and correctly append to `TokenLedger`.
