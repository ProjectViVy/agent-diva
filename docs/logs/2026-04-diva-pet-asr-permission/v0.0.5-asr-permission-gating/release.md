# Release

## Method

- Ship with the next `agent-diva-gui` desktop build.
- No migration step is required.

## Operator Notes

- If ASR is denied, users should grant microphone access in Windows privacy settings and any WebView/browser permission prompt, then toggle the microphone again.
- Persisted `asrEnabled` values are intentionally treated as disabled on startup to avoid background microphone activation without a fresh user action.
