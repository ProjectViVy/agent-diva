# Acceptance

1. Run `agent-diva onboard` on a fresh config and verify the default provider is `deepseek`.
2. With an existing config whose model is `deepseek-chat`, choose `minimax` in `onboard` and verify the model prompt no longer defaults to `deepseek-chat`.
3. Configure a provider model not present in the bundled list, while keeping `agents.defaults.provider` set, and verify startup/provider resolution does not fail.
4. Open the GUI chat screen and verify the model switch/config area shows the current provider label, including the default DeepSeek case.
5. Open GUI provider settings and verify selecting a provider no longer silently commits an invalid `provider + old model` combination before the user chooses a model.
