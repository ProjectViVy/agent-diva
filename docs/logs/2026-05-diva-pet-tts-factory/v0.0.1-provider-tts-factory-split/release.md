# Release

## Status

- No special release procedure required for this refactor-only iteration.

## Delivery Method

- Merge the code changes into the target branch.
- Include the updated GUI assets in the next normal desktop build/package cycle.

## Rollback

- Revert the `tts-service.ts` provider factory refactor and related unit test changes if provider-specific synthesis regressions are observed.
