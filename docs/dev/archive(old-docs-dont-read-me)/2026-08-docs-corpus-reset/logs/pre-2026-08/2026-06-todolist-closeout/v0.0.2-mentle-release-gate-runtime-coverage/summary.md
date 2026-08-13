# Mentle Release-Gate Runtime Coverage Summary

## Summary

This iteration closed the TODOLIST item for Story 6.5 Mentle release-gate coverage.

## Changes

- Replaced the static `MentleToolRuntimeConfig` gate test with AgentLoop-level runtime assembly coverage.
- Added an `AgentLoop::build_system_prompt()` read-only helper so release-gate tests can inspect the assembled governance prompt through the same runtime context boundary.
- Added enabled-runtime assertions that fail if default governance prompt assembly exposes `memtle_search`, `memtle_kg_query`, `memtle_*`, Mentle recall wording, or Palace Memory guidance.
- Moved the corresponding TODOLIST item from Open to Done.

## Impact

The release gate now exercises the higher-risk runtime surface where Mentle tools are available and `mentle_active()` can be true. Runtime behavior is unchanged except for the new public read-only prompt helper.
