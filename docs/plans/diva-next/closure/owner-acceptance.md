> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# Owner acceptance — DIVA Next closure candidate

This is the engineering handoff for the closure candidate. Product
acceptance is yours; a green build or this document is not acceptance.
Fill the **Result** column during the run; any `pending` engineering row
below stays pending until exercised on real hardware/credentials.

## 0. Identity of the thing you are testing

| Artifact | Pin |
| --- | --- |
| agent-diva | branch `feat/dn-closure-wave1`, HEAD `54be342c` (candidate manifest version `0.4.13`) |
| agent-vivy | `db7f0c558d4e` (branch `feat/dn-closure-wave1`) |
| laputa | `04b893243060` (branch `feat/dn-closure-wave1`) |
| Sealed library | `vivy-shared.so` sha256 `d0155e26fddbb4426f7ddc2ab82615980a174d002bfc57d84c9386eac03659e5` (Linux x64, 107,489,640 B) |
| Generation | `331bb89d975770edb9255e49c6d0e8496ed99f77a510bcb497733ec54d7e01f2`, spec `vivy.module/v1`, ABI v1, 21 modules, 20 `diva.cognitive.*` actions |
| Windows DLL | **not yet built** — no mingw/Windows toolchain in the dev VM; build with `sdk pack --recipe recipes/diva.vivy.yml --target shared` on a Windows machine, then record its sha256 here before testing |

A rebuilt candidate invalidates every row below; re-run from a new hash.

## 1. Environment setup (once)

1. Windows x64 machine with WebView2 runtime (present on Win10 1803+ /
   Win11; otherwise install the evergreen bootstrapper).
2. Build or obtain `vivy-shared.dll` + `vivy-shared.h` + `vivy_abi.h` +
   `generation.json` and stage them into
   `agent-diva-gui/src-tauri/vivy-runtime/` (same layout as the staged
   `.so`; `just shell-stage-runtime <pack-output>` performs this).
3. `pnpm install` in `agent-diva-gui/`, then `just tauri-build` (or
   `pnpm tauri build`) to produce the installer/bundle. Install into a
   clean account or wipe `%APPDATA%/…/DIVA` to simulate a fresh install.
4. Prepare your own provider credentials locally (SiliconFlow and/or
   MiniMax for speech; a chat model key for VIVY's configured provider).
   Keys are entered in Settings → they land in the OS credential store,
   never in the repo or this document.

## 2. Engineering evidence already on the candidate (Linux, .so)

Verified on the immutable candidate — do not re-prove these; spot-check
in the GUI:

- Real model turn completes; token totals and trajectory match the
  journal; restart replay is stable.
- Approval wait/resume works end to end (write_file, durable approval
  row, diff preview).
- `run/cancel` terminates truthfully; `VivyPollEvents` is bounded with a
  `gap` flag; GUI diagnostics persist and redact secrets.
- CI (on push): gates `legacy-calls` (9/9), backend boundary, vivy-bridge
  FFI tests; new `windows-shell-check` job compiles the full workspace on
  `windows-latest` and runs bridge tests.

## 3. Owner checklist (GUI, real provider, Windows)

Each row: what to do → what you must see → engineering state.

### Chat / runtime

| # | Scenario | Expected visible result | Eng. evidence |
| --- | --- | --- | --- |
| 1 | Fresh launch, create session, send a real message | answer streams in, token counters grow, trajectory/console reflects the turn | passed on candidate (Linux) |
| 2 | A run that asks for approval (e.g. file write) | approval prompt appears, approves → run resumes to completion; deny → run stops honestly | passed on candidate |
| 3 | Cancel a running turn | run ends cancelled, no phantom completion | passed on candidate |
| 4 | Hide window / reopen mid-run | same conversation continues, events still flowing, no duplicate owners | pending — needs live WebView2 |
| 5 | Quit during an active/suspended run | bounded teardown; on next launch the run is shown as cancelled (not resumed), no orphan approval | shutdown-cancels-suspended verified (OBS09-F4); GUI-side pending |
| 6 | Restart app | sessions/settings/diagnostics persist; totals do not inflate | passed on candidate |
| 7 | Image attach send | real bytes/MIME sent; unsupported file gets explicit UI error | wired (DN-2A); GUI spot-check |
| 8 | Regenerate selected turn / Start Goal | regenerate replaces in place; Goal entry works | wired (DN-2B); GUI spot-check |
| 9 | Child/subagent run | child appears linked under its parent run | pending — needs a live spawn |
| 10 | Recovery: crash during ambiguous effect | restart must NOT replay the effect; run reports recovery-required, never a silent duplicate | pending — not re-driven |

### Cognition (Garden)

| # | Scenario | Expected visible result | Eng. evidence |
| --- | --- | --- | --- |
| 11 | Persona initialize → edit → save → review | persona views show real content; review decide persists | actions verified (DN-4C) |
| 12 | Frozen vs current distinction | frozen panel shows the session snapshot; edits to persona do not retro-change it | seam verified via probe |
| 13 | **First turn on a fresh session** | ⚠ known gap OBS09-F2: a brand-new session's first turn may fail until a Frozen Core is captured — if you hit `internal error` on first turn, this is the reported gap (Bootstrap unwired), not a new bug | finding OBS09-F2 |
| 14 | Cognitive actions after app restart | ⚠ OBS09-F3: `module.action.invoke` may deny on sessions persisted from a previous process (-32009); creating a new session works | finding OBS09-F3 |
| 15 | ACTMEM/memory search/expand | scoped results or honest unavailable state | actions verified |
| 16 | Strategy/trigger toggle | trigger returns `disabled` until policy enabled | verified |

### Speech (needs your keys)

| # | Scenario | Expected visible result | Eng. evidence |
| --- | --- | --- | --- |
| 17 | Enter STT/TTS keys in Settings | stored in Windows Credential Manager; masked display | contract verified; OS store pending on Windows |
| 18 | Mic → STT → editable text | recognized text editable before send | wired; live accuracy is your call — use your own sample phrases |
| 19 | TTS speak reply (both providers) | plays synthesized audio for the fresh answer only | wired; live gates pending |
| 20 | Interrupt/stop speaking mid-playback | audio stops, stays stopped | generation-safe playback implemented |
| 21 | Hide window while speaking | audio does not auto-replay on reopen | pending — live window |
| 22 | Missing/invalid key | visible actionable error, not silent success | `credential_unavailable` path verified |

### Console / observability

| # | Scenario | Expected visible result | Eng. evidence |
| --- | --- | --- | --- |
| 23 | Token/usage view after turns | totals match what the journal recorded; coverage states honest (e.g. missing-usage counts visible) | passed on candidate |
| 24 | Trajectory/console view | real run/seq rows, bounded lists, single owner | passed on candidate |
| 25 | GUI diagnostics viewer | your appended records redacted and bounded | passed on candidate |
| 26 | Runtime log files | ⚠ OBS09-F1: the packaged candidate has no file log sink — runtime diagnostics surface empty is the recorded gap | failed row |

## 4. Known gaps handed to you (all recorded, none hidden)

- **OBS09-F1** no runtime file-log sink in the sealed `.so` —
  `diagnostics/logs source=runtime` is empty; rotation not exercisable.
- **OBS09-F2** FrozenCore capture seam unwired — fresh-session first turn
  fails until a core is captured; fix = wire `Bootstrap` at session bind.
- **OBS09-F3** cognitive actions are per-process authorized — persisted
  sessions deny `module.action.invoke` (-32009).
- **OBS09-F4** shutdown cancels suspended runs and drops pending
  approvals — no cross-process approval resume.
- **OBS09-F5** sandbox-denied promptable tool fails the run with a
  misleading "could not be paused" message (fail-closed).
- Whole-workspace `cargo check`/`clippy` unverified on Linux
  (GTK/WebKitGTK absent) — the new `windows-shell-check` CI job covers
  the Windows compile on every push.
- Dependency audit: 0 Rust vulnerabilities; 2 warnings
  (proc-macro-error unmaintained; glib unsound — Linux-only path);
  8 npm advisories all transitive/dev-facing (postcss/nanoid/pnpm/
  markdown-it) — recorded, not patched.

## 5. Diagnostic export procedure (bounded, safe)

If something fails, capture — in order:

1. In the GUI diagnostics/console view, screenshot the visible error.
2. Record the run id / approval id / session id shown (they match the
   journal).
3. If a terminal is available: the journal lives at
   `<VIVY home>/vivy.db` — `sqlite3 vivy.db "select seq,type from
   run_events where run_id='<id>' order by seq"` reproduces the exact
   event stream (payloads are redacted at write time, safe to share).
4. Copy GUI diagnostics via the bounded `diagnostics/logs` read —
  keys/secrets appear as `[REDACTED]`; nothing else needs exporting.

Do not zip arbitrary directories — the bounded reads above cover every
evidence need.

## 6. How to call it

- Accepted: every required scenario row passes on the Windows candidate.
- Accepted-with-residuals: pass rows pass; each failure names its OBS09 /
  TODOLIST disposition.
- Rejected: any parity claim above is visibly untrue, or a find-the-gap
  item (F1–F5) behaves differently than recorded.

