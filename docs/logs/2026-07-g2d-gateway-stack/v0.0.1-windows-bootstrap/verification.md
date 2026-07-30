# Verification

- CLI test targets compile.
- CLI clippy passes with warnings denied.
- Embedded Laputa clean-break gate passes.
- Real `agent-diva gateway run` remains alive after bootstrap.
- `/api/health` returns HTTP 200 with typed Memory ready, revision 2, two
  imported records and no degraded reason.
