# Acceptance

## User-facing acceptance steps

1. Start the GUI or desktop pet development entry.
2. Load a VRM character that has look-at support.
3. Play a VRMA motion through the existing animation controls.
4. Confirm the browser console no longer shows the deprecated `removeUnnecessaryJoints` warning.
5. Confirm the browser console no longer shows the missing `VRMLookAtQuaternionProxy` warning.
6. Confirm the browser console no longer shows missing i18n key warnings for `chat.attachFile`, `pet.voice.tts`, or `pet.voice.pushToTalk`.
7. Confirm app startup no longer logs the Vue `onUnmounted` active-instance warning.
8. Confirm the browser console no longer shows the `PCFSoftShadowMap` deprecation warning from project renderer setup.

## Known remaining item

- A `THREE.Clock` warning can still appear because it originates from `@sparkjsdev/spark@2.0.0`, not project runtime code changed in this iteration.
