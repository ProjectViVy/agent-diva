# C6 production cutover release

## Status

Not released and not merged to `dev`. The implementation is committed on the isolated
`feat/channel-epic-c6` worktree so the new runtime can be evaluated without creating a partial
dual-state release.

## Preview

From `C:\Users\Administrator\Desktop\morediva\agent-diva-channel-epic-c6`, use the normal external
configuration directory and start the foreground gateway:

```powershell
cargo run -p agent-diva-cli -- --config-dir <config-directory> gateway run
```

Runtime truth is available from `GET /api/channels/runtime`; the desktop channel settings page uses
the same endpoint. Do not send a real test message until the selected test account and recipient are
explicitly controlled.

## Rollback

Stop the preview gateway and return to the `dev` worktree. No production merge or push was made, so
rollback does not require configuration or database migration. Failed channel hot updates restore
the prior on-disk config and runtime set automatically.
