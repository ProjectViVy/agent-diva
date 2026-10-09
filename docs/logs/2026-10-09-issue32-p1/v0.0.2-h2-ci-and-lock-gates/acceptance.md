# Acceptance

## H2 status

**Implemented; local contract verified; native CI pending.** P1.2's failure
mode is closed in code and workflow configuration: desktop-only, speech-only,
lock-only, script-only, and GUI/binding changes trigger CI; the aggregate
requires active Go-host race tests, binding drift, boundary checks, sealed
pack/inspect, and existing transition checks. Both Linux and Windows jobs,
the VIVY transition guard, and the Windows shell job are required legs.

## Limits

- Workflow YAML was parsed locally, but GitHub Actions did not execute.
- Linux native race/pack and Windows native CGO/race results remain pending.
- No product acceptance, candidate approval, public upload, tag, or release is
  inferred from the local Python checks.
