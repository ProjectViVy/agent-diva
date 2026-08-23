# Acceptance

1. Memory add/update/remove still return `Applied` / `Failed` only.
2. `sync_turn` reports `Persisted` / `Noop` / `Failed` only.
3. Consolidation itemized writes no longer mention a proposed count.
4. Prompt still does not teach memory-write approval (`proposal_created`
   absent).
5. Skill distill still creates Evolution requests via `request_created`.
