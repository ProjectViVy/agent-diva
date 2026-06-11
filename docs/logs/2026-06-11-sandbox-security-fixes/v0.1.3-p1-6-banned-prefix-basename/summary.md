# P1-6 Banned Prefix Basename Matching

## Summary

- Reworked `is_banned_prefix()` to compare the basename of `argv[0]` instead of requiring an exact raw path match.
- Changed detection to true prefix matching so longer interpreter commands like `python3 -c ...` are still denied for auto-suggestion.
- Added regression tests for `/usr/bin/python3` and `/usr/local/bin/node` style aliases.

## Impact

- Dangerous interpreter and shell prefixes can no longer bypass the ban list just by using an absolute path alias.
