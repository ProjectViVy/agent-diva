# v0.4.9 acceptance — DN-6C

## Claims supported by evidence on this VM

- Voice controller fencing proven by unit tests: stale generations can
  never replay, quiesce releases recorder+player+pending native
  requests on every exit path, openResources drains to 0.
- STT drafts are editable in the composer and never auto-send.
- TTS auto-read admits only fresh primary runs (UI-started or live
  goal-activation); child-run terminals never notify; resync-replayed
  terminals excluded.
- Browser build reports native_unavailable honestly; no speech command
  executes outside the Tauri main window; no pet_* invocation exists.
- Speech settings: CAS conflict refreshes, transient key fields never
  echo stored secrets.

## Not proven here

- Real MediaRecorder getUserMedia + SiliconFlow/MiniMax live audio on a
  Tauri host — deferred to developer-native smoke; final acceptance
  remains the owner's on Windows x64.
