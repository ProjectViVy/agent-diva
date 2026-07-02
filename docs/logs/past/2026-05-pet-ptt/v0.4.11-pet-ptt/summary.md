# Diva pet push-to-talk summary

## Changes

- Replaced the standalone desktop pet push-to-talk recorder with the shared `useVoiceInput` ASR flow.
- Forwarded recognized desktop pet voice text to the main window via `desktop-pet-voice-message`.
- Added a matching push-to-talk button to the Diva voice panel, immediately to the right of the test voice button.
- Hid the voice test button from the Diva voice panel and pushed the push-to-talk button to the far right of the control row.
- Removed the push-to-talk pulse animation and changed its held state to red.
- Added a small Whispers header plus button that starts a new topic, inserts `让我们换个话题聊聊吧`, and triggers the same text through Diva TTS.
- Updated voice input support reporting so cloud ASR recording support is not hidden by missing Web Speech support.

## Impact

- Desktop pet voice input now transcribes and sends text to the active main chat session.
- Sidebar Diva pet users can trigger the same ASR-backed push-to-talk flow from the voice control row.
