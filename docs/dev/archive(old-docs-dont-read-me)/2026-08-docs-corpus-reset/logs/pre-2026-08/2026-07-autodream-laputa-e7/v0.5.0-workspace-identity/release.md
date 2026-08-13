# Release

The runtime computes workspace identity internally. New offline imports default
to the same canonical identity; the explicit `--workspace-id` option remains
available for controlled compatibility work.

For an existing raw-path typed store, typed startup performs the identity-only
upgrade after creating `.laputa/migrations/workspace-identity-v1/` with a
verified backup and phase manifest. Shadow mode still requires an existing
typed store and does not silently create one.

Rollback is available through:

`agent-diva-migrate memory identity rollback --workspace <path>`

No push or production deployment was performed.
