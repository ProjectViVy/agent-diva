# C6 production cutover release

## Status

Locally merged to `dev` as `8cc6580b` on 2026-09-03 after the user's explicit request. The merge
was not pushed. C6-D DTO removal and C6-E real-platform/desktop/MSRV acceptance remain open, so this
is a local product cutover rather than complete Epic closure.

## Preview

From `C:\Users\Administrator\Desktop\morediva\agent-diva`, use the normal external configuration
directory and start the foreground gateway:

```powershell
cargo run -p agent-diva-cli -- --config-dir <config-directory> gateway run
```

Runtime truth is available from `GET /api/channels/runtime`; the desktop channel settings page uses
the same endpoint. Do not send a real test message until the selected test account and recipient are
explicitly controlled.

## Rollback

Stop the gateway and revert merge commit `8cc6580b` with a new rollback commit if the local cutover
must be undone; do not rewrite shared history. No push or configuration/database migration was
made. Failed channel hot updates restore the prior on-disk config and runtime set automatically.
