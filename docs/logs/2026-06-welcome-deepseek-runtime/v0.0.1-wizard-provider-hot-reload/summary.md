# Summary

## Change

Fixed the welcome wizard DeepSeek setup path so saving a DeepSeek API key also writes the complete active provider selection:

- provider: `deepseek`
- API base: `https://api.deepseek.com/v1`
- model: `deepseek-chat`

The wizard now uses the same runtime update path as the provider settings screen, so the manager hot reloads the active provider immediately after first-run setup.

## Impact

This prevents the GUI from keeping an incomplete or stale provider runtime after the installation wizard. Previously, opening provider settings and saving again could repair the runtime because that path submitted the full provider/model/API base selection.
