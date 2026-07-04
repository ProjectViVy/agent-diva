# Release

## Delivery Method

- Standard Rust workspace source change; no special deployment step beyond the normal CLI release pipeline.

## Operator Notes

- Existing managed workspaces under `config_dir/workspaces/*` continue to work unchanged.
- Invalid path-like workspace names now fail fast with an explicit validation error.

## Rollback

- Revert commit `fix: harden workspace cli path handling` if this change needs to be backed out.
