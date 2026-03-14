# Summary

- Fixed the provider settings model selection flow so removing the currently active model from shortcuts no longer clears the runtime provider/model config.
- Fixed custom-model deletion so deleting the active custom model switches to a fallback model instead of saving an empty active model.
- Added a guard that refuses to delete the active custom model when no fallback model is available.

# Impact

- Prevents gateway startup failures caused by an empty `config.agents.defaults.model`.
- User-visible behavior changed in `Settings -> Providers -> Available Models`.
