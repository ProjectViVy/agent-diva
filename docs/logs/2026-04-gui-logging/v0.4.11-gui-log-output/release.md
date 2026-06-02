# Release

## Method

Ship with the next GUI build. No migration is required because the existing `logging` config schema is unchanged.

## Notes

- Relative `logging.dir` values continue to be interpreted relative to the Agent Diva config directory in the GUI log viewer.
- The GUI logger now writes to the same resolved directory.
