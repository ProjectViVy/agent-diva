# Acceptance

1. A `judge` assertion no longer silently passes when the judge path is active or unavailable.
2. A session over its token budget is rejected before the next provider call.
3. A cancelled or lost supervised run stops executing and records the correct terminal state.
4. Skill upload blocks injected content and only records `SkillLoaded` after success.
5. Inbound channel messages are sanitized for PII and blocked for injection before entering the main agent turn.
