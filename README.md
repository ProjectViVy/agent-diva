# Agent Diva

A Windows x64 Tauri/Vue desktop for the independently assembled Diva Rust core.
It includes durable chat, questions/tools/cancellation, local management and
close-to-tray hosting. Full release acceptance is still pending; see verification
limits below. No source checkout or development tools are needed by the installed app.

## Development

Install Node.js 24 LTS, npm, stable Rust, and the Tauri platform prerequisites:
https://v2.tauri.app/start/prerequisites/

On Windows, install the MSVC C++ build tools and WebView2 runtime.

```sh
npm ci
npm run import-core -- <explicit-core-artifact-directory>
npm run tauri dev
```

For a browser-only preview, run `npm run dev`. Native host calls are available
only inside the Tauri desktop application.

## Validation and packaging

```sh
npm run build
npm test
npm run test:artifact
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path bridge-tests/Cargo.toml --locked
npm run tauri build -- --bundles nsis
```

The core artifact must contain the Windows binary, Rust-generated TypeScript,
protocol examples/version, recipe, Cargo.lock, dependency notices and metadata
with hashes. Import verifies every file. Native builds fail if the binary or
contract differs, or the required pinned Rig revision is absent. Never copy a
backend source tree into this repository. CI runs frontend/bridge checks without
a core; the separate packaging workflow requires an explicit HTTPS artifact ZIP
and SHA256 and fails when it cannot obtain a matching artifact.

For native bridge integration tests, set `DIVA_TEST_CORE` to the explicit built
core executable, then run `cargo test --manifest-path bridge-tests/Cargo.toml --
--include-ignored`. Use `npm run dev -- --mode test --port 1422` only for the
synthetic IPC browser fixture; it does not talk to the core or use credentials.

## Installation and use

Run the NSIS installer as the intended Windows user. Installation is per-user.
WebView2 is required; the installer can download Microsoft's bootstrapper.
Data lives below `%LOCALAPPDATA%/org.projectvivy.agentdiva/`, separately under
`core-home`, `workspace`, `desktop-state`, `webview2` and `logs`. Upgrade and
ordinary uninstall retain this data. Unknown/corrupt state is rejected or shown
as an error, never silently cleared.

Open Settings → Providers, add an OpenAI/Anthropic-compatible endpoint and the
endpoint's exact model ID, choose an environment-variable credential reference
or enter a stored API key, then explicitly save. The key is write-only. Use the
explicit text/stream/tool test to check that endpoint; a successful local mock
does not establish real-provider compatibility. Select the default model and
return to chat. Model identifiers are not rewritten.

Closing the window hides it; the tray provides Show, Settings and Exit. Existing
Recap/wake work can continue while hidden. Exit allows ten seconds for shutdown,
then terminates the owned process tree. It does not cancel all durable tasks.
After a crash use Reconnect; unconfirmed sends are reconciled against committed
admissions. No task execution is promised after the entire application exits.
Python, Git Bash and MCP dependencies are optional for basic text chat. Python
initialization and MCP connections happen only on an explicit action. MCP values
reference host environment variables; the Python namespace receives only the OS
environment needed to run, not arbitrary credential variables.

## Verification limits

Offline/native-core tests cover durable admission, conflicts, history, cancellation,
typed management, outbox write failure, crash/reopen, competing session writers
and Job Object cleanup after forced host death. Vue runtime and MCP form tests,
production compilation and synthetic IPC browser checks pass. The browser fixture
was checked at actual CSS 500x400, 680/681px and 1100x760.

Short-path clean NSIS building, per-user installation, restricted-PATH startup,
reopening, forced host death, a test 0.1.0-to-0.1.1 upgrade and default uninstall
passed on the existing Administrator profile. Session/workspace data survived.
These do not establish native WebView2 layout at 125/150% OS scale, clean
standard-user installation or hidden Recap/wake behavior. Local native UI
automation is unavailable; WebView2 was already installed on this test machine.

The authorized Qwen gateway returned the exact model ID `qwen3.8-omni-flash` from
its model-list endpoint. Native streamed chat with an actual read-tool call and
committed result passed, as did the explicit text, stream and tool probes. The
mixed-case ID supplied in the local note returned model-not-found; original files
were not changed. The separate StepFun endpoint failed HTTP transport here.

## Layout and scope

- `src/`: Vue application and native host connection state.
- `src-tauri/`: Rust desktop entry point, commands, capabilities and packaging.
- `.github/workflows/ci.yml`: Windows build validation.

The desktop consumes only the explicitly imported core artifact. Runtime never
uses the backend checkout, Node, Cargo or the legacy Go implementation. No arbitrary
process-execution IPC is exposed to the UI. Default icons remain Tauri placeholders.
