# Story 1.1 Release Notes

## Release Method

No separate deployment action is required for this story. The change is a Rust library domain-model addition and will ship with the next workspace build/release.

## Operator Impact

- Downstream crates can import shared governance types from `agent_diva_core::evolution`.
- No config migration, data migration, service restart behavior, or user-facing workflow changes are introduced by this story.
