# W0-3 Summary — Windows x64 leg, round 2 (upstream F1/F2/F3 fixes)

Windows Server 2022 (Build 20348), interactive desktop session, native
Windows. Executor: Devin session
`devin-ae03047a0aea482196c02fa73c416630` on behalf of mastwet.

## Source pins

| repo | ref | commit |
|------|-----|--------|
| agent-diva | `feat/wails-go-host` | `780237a5` (incl. upstream `8db11f51` tray fix + `f833f4eb` repin + w0-2 commits) |
| agent-vivy | `feat/wails-migration` | `42c263f2` (pack overlays built dist into staged host — the F1 fix) |
| laputa | detached | `6f2eed2d71c8` |

Toolchain unchanged from w0-2: Go 1.26.8, Node 24.0.1, wails3
v3.0.0-beta.27, WebView2 154.0.4258.53.

## Sealed candidate (final, clean build)

`python scripts/build-desktop.py --mode build --development --output
repos/artifacts/diva-go-host-r2` →

- `diva.exe` sha256 `dfc85ae1e962b46e7bb8ada8aa555022b9d7d110b34f29fbb242c25f946683fa`
- generationId `8616618d1cde3914c85d5bd7b42a00e4b5dd3ddafc8ef4bbaf8dfa24639f5e30`
- build-report: host 780237a5, vivy 42c263f2, laputa 6f2eed2,
  go1.26.8, wails v3.0.0-beta.27, windows/amd64, release=false
- An intermediate build of the same tree plus a temporary
  `internal/rpc/control.go` debug patch (error detail passthrough,
  reverted before this build) produced `diva.exe` sha256 `be4d01b4…`
  and was used to surface the real F5 error text; all other findings
  reproduce identically on the clean artifact.

## Verified fixed (regression pass)

- **F1 — embedded UI**: the Vue frontend now renders in the sealed exe
  (greeting, chat surface, sidebar, model selector; screenshots
  `clean-build-ui-loaded.png`). Pack overlays built host assets into the
  staged tree (`agent-vivy 42c263f2`).
- **F2/F3 — tray reopen + quit**: tray icon now has AttachWindow +
  Show/Quit menu. Verified live: left-click toggles hide→reopen; menu
  "Show" reopens, "Quit" exits and releases the organism lease
  (immediate relaunch accepted). `W5-T4-HIDE-REOPEN` → passed.
- **W5-T4-WEBVIEW-RELOAD** → passed: F5 reload re-renders the real UI;
  runtime unaffected.

## New findings (real, upstream-blocking)

### F5 — sealed `turn/start` hard-fails on every fresh session (root-caused)

Every GUI send on a session without a captured frozen core fails:

```
internal error: runtime: prepare session authority:
no captured frozen core for this session
```

Chain (verified in source + live):

1. `internal/runtime/service.go:1014` — `Prepare` calls
   `bundle.Prepare` for the primary run.
2. `internal/modules/diva-cognitive/factory.go:111` — `BindHumanSession`
   → `human.ReadFrozen`.
3. laputa `garden/agentapi/embedded_domain.go:719` —
   `personactx.Store.Get` is **read-only**; the only writer is
   `Store.Capture` reachable via `SessionProvider.Get` inside
   `recall.FastService` (`BoundClient.FastRecall`/`Bootstrap`).
4. Nothing on the agent-vivy path ever calls it —
   `frozen_core_sessions` in `~/.vivy/garden/garden.db` stays at 0 rows
   for every GUI-created session.

Dev paths (headless `vivy run`, embedded probe) are unaffected because
`cognitiveBundle` is nil outside the sealed assembly (`control actions
disabled: sealed Generation identity unavailable`).

Workaround proven: `persona.Service.Initialize` (5 persona docs) +
`SessionProvider.Get(sessionID)` lazy-capture → subsequent `turn/start`
accepted and the run completes end-to-end through the mock provider
(runs `run_42757e53…`, `run_7498584d…`, `run_ff97857f…` → `completed`,
assistant messages persisted in `vivy.db`).

### F6 — GUI never binds or renders session messages

- After restart, `vivyChat.connect()` never reaches
  `loadSession(latest)`: status stays "连接中" (projection `connecting`,
  `ready` never set) and `currentSessionId` remains unset — proven
  because a subsequent send created a **new** session
  (`sess_4d13a567743cd881`) instead of resuming `sess_303a…`.
- Backend is healthy: the run journal records
  `run.started → model.request → model.delta → … → run.completed` and
  user/assistant messages persist (`vivy.db` verified). All connect-path
  RPCs (`session/list`, `review/list`, `approval/list`, `question/list`,
  `session/get`, `session/messages`, `run/subscribe`) succeed when
  driven against the same dispatch table via the embedded probe — the
  defect is sealed-host- or GUI-bridge-specific and needs a WebView2
  devtools trace (no remote-debugging surface available in this build).
- Reproduces identically on the clean final artifact.

### Earlier residual — empty `memory.sqlite3`

`~/.vivy/memory/memory.sqlite3` is created but stays 0 bytes across
turns; expected memory writes never land (consistent with F5 gating
every real turn).

## Acceptance deltas (wails-candidate-acceptance.json)

| row | w0-2 | w0-3 |
|-----|------|------|
| W5-T4-HIDE-REOPEN | failed | **passed** |
| W5-T4-WEBVIEW-RELOAD | pending | **passed** |
| W5-T2-FIRST-TURN | pending | **failed** (F5) |
| W5-T3-CHAT-MATRIX | pending | **failed** (F6) |
| W5-T3-COGNITIVE | pending | pending (persona init + frozen capture proven via module-level probe; matrix drive blocked by F5/F6) |
| W5-T4-EVENT-LOSS | pending | pending (`run/subscribe` verified; live GUI event lane blocked by F6) |
| W5-T2-GRANTS-CATALOG | pending | pending (tools/list + governance catalog verified on shared dispatch; sealed GUI drive blocked by F6) |
| W5-T5-VOICE-REAL | pending | pending (no audio endpoints on this VM — unchanged) |
| W5-WINDOWS-MATRIX | pending | pending (rollup updated) |

## GO/NO-GO

- F1/F2/F3 fixes: **verified** — UI embedded, tray reopen/quit work.
- Candidate chat path: **NO-GO** — F5 blocks every first turn; F6 means
  the GUI cannot display a session at all. Both are platform-independent
  upstream defects found on real Windows evidence.
