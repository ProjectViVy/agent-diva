# Acceptance

## User Steps

1. Start Agent Diva GUI.
2. Open the Diva Pet view.
3. Click the microphone control.
4. Grant microphone permission when prompted.
5. Confirm the voice panel enters listening state and ASR no longer logs repeated `not-allowed` errors.

## Denied Permission Path

1. Click the microphone control.
2. Deny microphone permission.
3. Confirm ASR remains disabled and the UI shows a microphone permission error.
4. Grant microphone access in system settings, then click the microphone control again.
