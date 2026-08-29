# HQ-02 Agent Dispatcher Summary

## Completed

- Added a session-aware dispatcher over the HQ-01 bounded admission kernel. Every Bus/direct turn now acquires a canonical `channel:chat_id` lease before circuit, rate, provider, tool, session, or Memory work.
- Isolated mutable session/turn fields in `SessionWorkerState`; dispatcher/kernel state remains process-shared and never holds a lock across turn execution.
- Made inbound approval policy and subagent mask inheritance immutable turn snapshots. Supervised background runs persist their captured mask instead of reading another turn's global mask.
- Connected Stop to the running cancellation token and Reset/Delete to both the running token and queued waiters.

## Compatibility boundary

Production Bus consumption remains globally serialized in HQ-02. The dispatcher supports independent session workers concurrently, but Manager streams still correlate only by channel/chat ID. HQ-03 must project request/trace identity before production transport concurrency is enabled.

## Commits

- `9302f8e7 feat: wire bounded session turn dispatcher`
- `b53c619f refactor: isolate session worker turn state`
