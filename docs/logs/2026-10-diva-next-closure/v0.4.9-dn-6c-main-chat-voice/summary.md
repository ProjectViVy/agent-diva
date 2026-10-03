# v0.4.9 — DN-6C main-chat online speech wiring (backfilled log)

Scope: `docs/plans/diva-next/closure/DN-6C.md`. Committed as `f993afc4`.
This log directory was missed at commit time and is backfilled with
v0.4.10; content is reconstructed from the commit and SDD ledger.

## What changed

`agent-diva-gui` frontend only (no new backend code):
- `features/voice/useVoiceController.ts` — dep-injected controller:
  minted identities (request_id/utterance_id), generation fencing from
  `window_context`, `quiesce()` on every exit path, single fenced
  playback path. Tests: sttDraftNeverAutoSends, lateMp3NoPlayback,
  reloadReadsGeneration, onlyFreshPrimaryAutoReads,
  allExitPathsReleaseAudio, stale synth never restarts.
- `features/voice/encodeWav.ts` — decode → downmix → 16kHz linear
  resample → PCM16 → RIFF; rejects >120s/>8MiB.
- `voiceRecorder.ts` / `voicePlayer.ts` — live MediaRecorder / Blob-URL
  adapters; tracks released on every exit; Blob created only after
  fenced bytes.
- `platform/desktop-host.ts` — sole literal-command seam; 13 speech
  command wrappers with exact wire shapes (Raw body + meta headers,
  bare-string asset delete, full-config readback on update).
- `api/speech.ts` — typed facade; `NativeUnavailableError` under
  browser; settings surface returns full SpeechConfigReadback.
- `state/voice.ts` — singleton wiring: conversation invalidate →
  quiesce, fresh-reply hook → fenced auto-read, speech:diagnostic →
  OBS-08 recorder.
- `state/vivy-chat.ts` — fresh-primary run fencing: marks runs started
  by this UI (turnStart/sessionEdit) and live work.current_run_id;
  `child.started` with non-empty parent_run_id tags child runs whose
  terminals never notify; needsResync excluded; terminal = one-shot.
- `ChatView.vue` — mic/Stop/transcribing states, editable draft
  (never auto-send), Volume2 replay per agent message, error surfacing.
- `SpeechSettings.vue` + settings mount — providers, transient
  credentials, assets, CAS config update.
- locales EN/ZH + styles.

## Rulings
- Fresh-primary admission: only UI-started runs or live
  work.current_run_id may auto-read; replayed run.completed for
  resync-subscribed runs excluded; child terminals never notify.
- Invalidation order: stop local media first, cancel pending native
  requests, then advance/ack native context.
- `current_run_id` comes from Service.GoalActivation (live goal loop) —
  marking it fresh-primary is honest.
