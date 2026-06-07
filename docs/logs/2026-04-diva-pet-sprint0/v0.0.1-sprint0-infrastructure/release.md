# Release — Sprint 0

## Deployment Method

Not applicable — Sprint 0 is an intermediate milestone with no standalone release artifact. Changes are feature-gated behind `pet.enabled` config flag (default: `true` for development, adjustable per environment).

## Rollback

To disable Diva Pet sidebar:
1. Open browser DevTools → Application → Local Storage
2. Set `agent-diva-pet-config` to `{"enabled":false,"renderer":"vrm","vrmModel":"","ttsEnabled":false}`
3. Refresh — sidebar entry disappears

## Version

v0.0.1-sprint0-infrastructure (internal milestone, no version bump)
