# Release

No deployment or remote push was performed.

The change is committed locally as `fix(core): stabilize supervised run finalization`. It is backward-compatible at the public API level; the behavioral change is that executor ticks now report terminal persistence failures instead of silently succeeding.
