# Acceptance

1. Open the GUI settings provider page and select a non-DeepSeek provider such as OpenAI or Anthropic.
2. Enter an API key and confirm the status card switches from `需要配置` to `已就绪` after save/refresh.
3. Switch from DeepSeek to another provider without manually retyping the model and confirm the selected model is normalized to that provider instead of keeping a stale DeepSeek model.
4. Restart the app and confirm the provider page still shows the chosen provider rather than snapping back to DeepSeek.
5. Use the titlebar model selector and confirm the active provider/model pair matches the settings page selection.
6. Run a minimal chat request and confirm the request uses the selected provider path instead of a DeepSeek fallback.
