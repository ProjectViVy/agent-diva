# Story 2.2 Proposal Inbox Release

## Release Method

- No deployment was performed in this iteration.
- The change is ready for code review after the targeted GUI test pass.

## Release Notes

- Evolution Inbox now supports dense review rows, filters, local read/defer markers, legal batch actions, and keyboard shortcuts.
- Defer is intentionally UI-local until the backend adds a durable proposal state or snooze contract.

## Known Release Blockers

- GUI build remains blocked by pre-existing TypeScript errors outside this story's changed files.
- Full GUI test remains blocked by unrelated SubAgentPanel and DivaPetView test setup failures.
