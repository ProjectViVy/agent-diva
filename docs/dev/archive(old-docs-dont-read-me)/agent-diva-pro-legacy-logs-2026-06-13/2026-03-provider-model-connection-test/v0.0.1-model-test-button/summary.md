# Summary

- Added a per-model `Test Connection` action to the GUI provider settings model list.
- The action is hover-revealed by default and keeps visible while a test is running or after a result is returned.
- Custom models now show the action order `test -> delete`, so the new button sits immediately to the left of the delete button.
- Implemented a new Tauri command `test_provider_model` that builds a lightweight provider client and runs a single-turn `agent-diva-neuron` connectivity probe against the selected model.
- The probe uses the current provider form state (`provider`, `model`, `api_base`, `api_key`) without mutating saved config.

# Impact

- User-visible GUI behavior changed in `Settings -> Providers -> Available Models`.
- Tauri bridge surface expanded with a new `test_provider_model` command and result DTO.
