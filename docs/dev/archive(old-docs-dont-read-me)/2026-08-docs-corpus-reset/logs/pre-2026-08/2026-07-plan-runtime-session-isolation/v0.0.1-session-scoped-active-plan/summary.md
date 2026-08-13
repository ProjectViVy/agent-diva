# Session-Scoped Active Plan Runtime

Active plan restoration now selects reports only from the current GUI session. This prevents a pending or approved plan in one conversation from appearing in a normal Agent conversation.

The Tauri command accepts an optional `sessionKey`; GUI callers always provide the current session key for restoration and plan revision feedback.
