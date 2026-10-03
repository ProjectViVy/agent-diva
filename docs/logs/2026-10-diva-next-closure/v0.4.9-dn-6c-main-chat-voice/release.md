# v0.4.9 release — DN-6C

One commit on `feat/dn-closure-wave1` (already landed):

- `f993afc4 feat(voice): connect online speech to main chat`
  (20 files, +1667/−10)

New: `features/voice/{useVoiceController,encodeWav,voiceRecorder,
voicePlayer}.ts` (+tests), `api/speech.ts`(+test), `state/voice.ts`,
`components/settings/SpeechSettings.vue`,
`.superpowers/sdd/dn-6c-main-chat-voice/progress.md`.

Modified: `platform/desktop-host.ts`, `state/{vivy-chat,vivy-run-
messages,chat-message}.ts`, `ChatView.vue`, `SettingsView.vue`,
`SettingsDashboard.vue`, `styles.css`, `locales/{en,zh}.ts`.
