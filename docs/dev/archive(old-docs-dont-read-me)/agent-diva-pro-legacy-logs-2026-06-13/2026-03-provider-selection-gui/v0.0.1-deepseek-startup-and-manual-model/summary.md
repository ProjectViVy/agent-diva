# Iteration Summary

## Changes

- Fixed GUI startup model shortcut hydration so the current runtime provider/model is automatically represented in `savedModels`.
- Fixed the main titlebar model switcher so default DeepSeek appears without requiring the user to reselect `deepseek-chat` from provider settings.
- Added manual model entry in GUI provider settings, allowing users to add a provider-owned model ID that is not present in the bundled or live catalog.
- Manual model entry now performs "save and switch" in one action so the new model becomes current immediately and is available from the titlebar shortcut list.
- Tightened titlebar selection state to match on both provider and model, preventing wrong highlighting when different providers expose same model names.

## Impact

- User-visible GUI behavior changed on startup and in provider settings.
- No backend API contract or Rust provider catalog interface was changed.
