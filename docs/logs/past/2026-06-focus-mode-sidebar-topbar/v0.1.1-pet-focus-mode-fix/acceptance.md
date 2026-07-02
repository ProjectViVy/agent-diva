# Acceptance

## User-Facing Checks

- Navigate to any non-pet page such as Chat: the regular topbar is visible.
- Navigate to Pet: the topbar is not rendered.
- On Pet, the pet content fills the main window height without reserving topbar space.
- On Pet, click the top-left pet menu button: the sidebar expands.
- With the sidebar expanded on Pet, the topbar remains hidden.
- Click the pet menu button again: the sidebar collapses back to pet focus layout.
- Leave Pet for Chat or Settings: the regular topbar returns.

## Acceptance Status

Automated component acceptance passed via `NormalMode.test.ts`. Visual in-app Browser smoke is pending because the browser backend was unavailable in this environment.
