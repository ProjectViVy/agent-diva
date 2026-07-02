# Summary

- Added explicit `agents.defaults.provider` support so provider selection no longer depends entirely on model-name guessing.
- Changed the default workspace configuration to DeepSeek with `provider = "deepseek"` and `model = "deepseek-chat"`.
- Fixed CLI `onboard` so provider choice drives model choice instead of reusing the prior global model default.
- Fixed GUI startup/config state so the chat view can display the current provider, including the default DeepSeek case.
- Kept unknown provider-owned models usable when the provider is explicit, instead of failing resolution because the model is absent from the bundled list.
