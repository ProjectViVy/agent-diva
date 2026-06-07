# v0.0.3 Debug Gateway Bundle Release

## Release Method

- Ship as a normal workspace commit.
- No migration is required.
- No GUI packaging change is required.

## Operator Notes

- Start debug mode with `agent-diva gateway run --debug`.
- Create a local bundle with `agent-diva gateway bundle` or `agent-diva gateway bundle --run-id <id>`.
- Treat generated debug run directories and bundles as sensitive artifacts.
