# Verification

## Commands

- `pnpm build`
  - Result: Passed.
  - Notes: Vite reported existing large chunk warnings after successful build. Re-run after i18n, lifecycle cleanup, and shadow-map changes also passed.

- `pnpm test`
  - Result: Passed.
  - Notes: 22 test files passed, 303 tests passed. Re-run after follow-up warning cleanup also passed.

## Manual smoke test

- Not run in this iteration.
- Suggested path: start the desktop pet or embedded pet entry, load a VRM model with a VRMA animation, and observe the browser console.

## Expected runtime observation

- `VRMUtils.removeUnnecessaryJoints` deprecated warning should no longer appear from project code.
- `createVRMAnimationClip: VRMLookAtQuaternionProxy is not found` should no longer appear when the VRM has `lookAt`.
- `chat.attachFile`, `pet.voice.tts`, and `pet.voice.pushToTalk` missing-locale warnings should no longer appear.
- Vue `onUnmounted is called when there is no active component instance` warning from app startup cleanup should no longer appear.
- `THREE.WebGLShadowMap: PCFSoftShadowMap has been deprecated` should no longer appear from project code.
- `THREE.Clock` warning may still appear from `@sparkjsdev/spark`.
