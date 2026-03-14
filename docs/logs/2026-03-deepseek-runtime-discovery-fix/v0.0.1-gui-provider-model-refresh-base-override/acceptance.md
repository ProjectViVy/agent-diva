# Acceptance

1. Open the GUI settings and enter the provider page.
2. Ensure a stale `providers.deepseek.api_base` value exists in config, such as `https://api.xiaomimimo.com/v1`.
3. Select `DeepSeek` in the provider list.
4. Click `Refresh online models`.
5. Confirm the request result references `https://api.deepseek.com/v1/models` instead of the Xiaomi Mimo endpoint.
6. Confirm the returned model list or error now belongs to DeepSeek only.
