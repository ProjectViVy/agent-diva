# M3 Agent Loop Image Assembly Release

## Release Method

This is an internal runtime assembly change within the agent loop. Release is through the normal Rust workspace build and validation pipeline.

## Deployment Notes

- No database migration is required.
- No provider configuration change is required.
- No GUI change is included in this iteration.
- Downstream provider adapters still need the later M5 work to serialize structured image parts into provider-specific request JSON.
