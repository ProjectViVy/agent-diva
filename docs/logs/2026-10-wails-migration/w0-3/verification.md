# W0-3 Verification — commands and observed output

All commands ran on Windows Server 2022 (MSYS/Git Bash +
PowerShell). Profile root `C:\Users\Administrator\.vivy`; mock provider
`tests/ffi/mock_provider.py` wrapper on 127.0.0.1:18321 with
`DEEPSEEK_API_KEY=mock-key VIVY_API_BASE=http://127.0.0.1:18321
VIVY_PROVIDER=deepseek`.

## Rebuild

```
cd repos/agent-diva
python scripts/build-desktop.py --mode build --development \
  --output C:\Users\Administrator\repos\artifacts\diva-go-host-r2
# sealed go-host artifact: .../diva-go-host-r2
# build-report: host 780237a5, vivy 42c263f2, laputa 6f2eed2
# generationId 8616618d1cde3914c85d5bd7b42a00e4b5dd3ddafc8ef4bbaf8dfa24639f5e30
Get-FileHash diva.exe → dfc85ae1e962b46e7bb8ada8aa555022b9d7d110b34f29fbb242c25f946683fa
```

## F1 regression — embedded UI loads

Sealed exe launched → DIVA window renders the real Vue frontend
(chat surface, greeting, sidebar nav, model selector "deepseek-chat"),
not the w0-2 `.gitkeep` directory listing. Screenshot:
`clean-build-ui-loaded.png`.

## F2/F3 regression — tray reopen + quit

```
Hide via window X → process + runtime stay alive.
Tray left-click → window reopens (same session state).
Tray right-click → menu {Show, Quit}; "Show" reopens, "Quit" exits.
After Quit → organism lease released → immediate relaunch accepted
(no -32086 rejection, no TTL wait).
```
Screenshots: `tray-menu.png`, `tray-reopen.png`.

## F5 — frozen-core gate (root cause)

GUI send on a fresh session (debug build with
`internalError` passthrough, since reverted):

```
错误: VivyCallError: internal error: runtime: prepare session
authority: no captured frozen core for this session
```

Screenshot: `f5-frozen-core-error.png`. A new session row
`sess_4d13a567743cd881` was created; **no run row** — `turn/start`
rejected before run creation.

Source chain:

- `internal/runtime/service.go:1014` → `bundle.Prepare`
- `internal/modules/diva-cognitive/factory.go:111` → `BindHumanSession`
  → `human.ReadFrozen`
- `garden/agentapi/embedded_domain.go:719` → `personactx.Store.Get`
  (read-only)
- sole writer `Store.Capture` via `SessionProvider.Get` inside
  `recall.FastService` (`garden/internal/runtimecore/runtime.go:160`)
- zero callers on the agent-vivy path → `frozen_core_sessions` = 0 rows:

```
sqlite3 ~/.vivy/garden/garden.db \
  "select count(*) from frozen_core_sessions" → 0   (fresh sessions)
```

Dev/embedded host unaffected: `cognitiveBundle` nil → Prepare skipped
(`WARN control actions disabled: sealed Generation identity
unavailable`).

## F5 workaround + mock-provider turn (proven)

`laputa/garden/cmd/frozencap` probe (internal module access, untracked):
`persona.Open` → `persona.Initialize` (5 docs:
identity/relationship/redline/user/world) → `personactx.OpenStore` →
`SessionProvider.Get(ctx, sessionID)` → row captured.

```
frozen_core_sessions: 1 row for sess_303a03d061266b5d / sess_b5170a2b88a47af1
GUI sends → turn/start accepted → runs complete:
run_42757e531e357baa  sess_b5170a2b88a47af1  completed
run_7498584dfb67a79e  sess_b5170a2b88a47af1  completed
run_ff97857f754ef296  sess_303a03d061266b5d  completed
mock_provider log: MOCKREQ call=5/6 model=deepseek-flash stream=...
vivy.db messages: user + assistant ("done") persisted
vivy.log audit: run.started → model.request → model.delta →
  model.usage → model.call.finished → model.completed → run.completed
```

## F6 — GUI binding/render defect

- Fresh restart → `vivyChat.connect()` → status stays "连接中";
  greeting only; persisted session messages for the latest session
  (`sess_303a…`, updated_at newest) never render.
- Send after restart created `sess_4d13a567743cd881` (proves
  `currentSessionId` unset — `loadSession` never ran or never bound).
- No user echo and no assistant bubble for completed runs (F5-gated
  sends show the error bubble correctly — error lane works).
- All connect-path RPCs verified OK via embedded probe against the same
  dispatch table (`initialize`, `review/list`, `approval/list`,
  `question/list`, `settings/get`, `settings/providers`, `tools/list`,
  `session/list`, `session/get` returning persisted messages,
  `session/messages`, `session/todos`, `run/subscribe` → `sub_…`;
  `session/work` = -32601 expected, GUI tolerates).
- `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333`
  produced no CDP endpoint in this build — remaining diagnosis needs a
  devtools-capable build or GUI-side logging.

## RPC probe harness (local, uncommitted)

`agent-vivy/cmd/turnprobe` — `embedded.Open` → `session/create` →
`session/set_permission{smart}` → `turn/start` → method sweep.
`laputa/garden/cmd/frozencap` — persona init + lazy frozen-core capture.
Both kept out of commits; a temporary `control.go` error-detail patch
was reverted before the final build.
