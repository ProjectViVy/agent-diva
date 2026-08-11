# CTX-C3 acceptance

1. Run a tool that returns at most 12,000 sanitized characters and confirm its tool message remains inline.
2. Run a tool that returns more than 12,000 characters and confirm the tool message is a `ToolResultRef` containing an opaque `ta_v1_...` ID, matching tool-call metadata, size, SHA-256, preview, and `truncated: true`.
3. Call `read_tool_result` with that ID and a valid `[start,end)` range no larger than 12,000 characters; confirm the exact sanitized range is returned.
4. Restart the gateway without deleting the session and confirm the same ID remains readable.
5. Try the ID from another session, a forged/path-like ID, and an invalid range; confirm stable fail-closed errors without filesystem paths or original content.
6. Build a long turn with older tool results and exceed the C2 inline-result soft limit; confirm only old completed successful results larger than 4,000 characters become refs, while the current tool group, errors, current user content, and assistant/tool-call pairing remain intact.
7. Delete the session and confirm subsequent reads return `artifact_missing`.
