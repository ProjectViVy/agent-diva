# P0-1 Shell Command Injection

## Summary

- Reworked `SandboxCommand::to_command_string()` to emit safely quoted command strings instead of rejoining raw arguments.
- Added platform-specific quoting for POSIX shells and PowerShell so metacharacters remain data, not syntax.
- Added regression tests covering metacharacter preservation and direct execution paths.

## Impact

- Arguments containing shell metacharacters such as `;`, `|`, `&`, `$`, `` ` ``, `<`, and `>` are no longer reinterpreted as extra shell syntax when built from `SandboxCommand`.
