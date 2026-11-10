# Agent Diva

<img src="https://raw.githubusercontent.com/ProjectViVy/agent-diva/c565bb245cc920258d7f8c7fcd9544fbba545af7/docs/resources/diva.png" alt="Agent Diva illustration" align="right" width="300" />

[简体中文](README.zh-CN.md) · [Project documentation](https://projectvivy.github.io/agent-diva-docs/) · QQ group 788599177

Agent Diva is a Windows x64 desktop AI assistant from Project ViVy. Its Tauri and Vue app runs an independently built Diva Rust core and provides streaming chat, persistent local sessions, and a workspace for tools, skills, and persona.

The name comes from [Vivy: Fluorite Eye’s Song](https://en.wikipedia.org/wiki/Vivy:_Fluorite_Eye%27s_Song). **Agent** refers to the executor before self-awareness, and **Diva** is the singer at the center of the stage. The project grew out of the original Agent Diva Rust workspace and its nanobot-inspired agent loop.

<br clear="both" />

## Quick start

1. Run the Windows x64 NSIS installer under your own account. You can [build the installer](#validate-and-package) with a matching core artifact. Installation is per-user. WebView2 is required and the installer can download Microsoft’s bootstrapper. The installed app includes the core.
2. On first launch, choose a provider, enter its API key or environment-variable reference, and save. Continue after the text, streaming, and tool checks pass.
3. Describe the assistant you want or select the built-in persona. You can edit its files later under **Settings → Laputa Persona**.
4. Initialize the Python workspace and optional local MemPalace memory, or defer setup to **Settings → Runtime**. Select **Start chatting** to open the conversation.

For an existing installation, use **Settings → Providers** to edit OpenAI- or Anthropic-compatible endpoints or select a default model. Enter the exact model ID advertised by the endpoint. API keys entered in the app are write-only. Curated provider choices include DeepSeek, MiniMax, and Alibaba Bailian. Bailian offers China standard, international standard, and China Coding Plan variants.

## What you can do

- Stream responses, inspect tool calls and results, answer questions from the assistant, cancel a turn, and reopen saved sessions.
- Manage local skills, managed skills, MCP connections, and the six core persona files in Settings. Recap and Tasks also have dedicated panels.
- Initialize Python and connect MCP servers explicitly. MCP configuration can reference host environment variables. The Python process receives a restricted OS environment.
- Close the window to hide it in the tray. The tray offers Show, Settings, and Exit. Existing Recap and wake work can continue while the app is running in the tray. Exit gives the core ten seconds to shut down before terminating its owned process tree. Use **Reconnect** after a crash to reconcile unconfirmed sends with saved submissions.

The bundled core starts with `--approve` to trust resources in the fixed desktop workspace. Other assistant questions appear in the chat.

Desktop data is stored under `%LOCALAPPDATA%/org.projectvivy.agentdiva/`, with separate `core-home`, `workspace`, `desktop-state`, `webview2`, and `logs` directories. Ordinary upgrades and uninstall retain that data.

## Develop from source

Install Node.js 24 LTS (24.15.0 or later within the 24.x line), npm, stable Rust, the MSVC C++ build tools, and WebView2. See the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
git clone https://github.com/ProjectViVy/agent-diva.git
cd agent-diva
npm ci
npm run import-core -- "<path-to-extracted-core-artifact>"
npm run tauri dev
```

Use a matching Windows x64 core artifact from the core build. Pass the extracted directory containing `metadata.json`. The importer verifies hashes and the pinned Rig revision. The artifact includes the core executable, Rust-generated TypeScript, protocol examples and version, the build recipe, Cargo.lock, and dependency notices. The Vue UI calls the Tauri host through IPC. The host launches the packaged core and exchanges JSON frames over its standard input and output.

Run `npm run dev` for a browser-only frontend preview. For synthetic IPC, use `npm run dev -- --mode test --port 1422`. The fixture runs with simulated core responses and needs no credentials.

## Validate and package

The [CI workflow](.github/workflows/ci.yml) runs the following checks without a core executable.

```sh
npm run build
npm test
npm run test:artifact
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path bridge-tests/Cargo.toml --locked
```

To exercise the bridge with a real core, set `DIVA_TEST_CORE` to the matching executable and run `cargo test --manifest-path bridge-tests/Cargo.toml --locked -- --include-ignored`.

After importing the core, run `npm run tauri build -- --bundles nsis`. The installer appears under `src-tauri/target/release/bundle/nsis/`. The [packaging workflow](.github/workflows/package.yml) accepts an explicit HTTPS URL for the core ZIP and its SHA-256 checksum.

Full release acceptance is pending. Upstream verified per-user installation, startup with a restricted PATH, crash recovery, and data retention across an upgrade and uninstall on an existing Administrator profile. Clean standard-user installation, native WebView2 layout at 125% and 150% display scaling, and Recap/wake behavior while hidden still need release verification. Provider compatibility depends on the endpoint and model you test.

See the [upstream verification record](https://github.com/ProjectViVy/agent-diva/blob/8468ca94eb95d3e4e3dc493c6419a4b47431dd63/README.md#verification-limits) for the tested environments and endpoints.

## Repository layout

| Path | Contents |
| --- | --- |
| [`src/`](src/) | Vue desktop UI, host connection state, and generated protocol |
| [`src-tauri/`](src-tauri/) | Rust desktop host, core process supervision, local state, and NSIS configuration |
| [`scripts/`](scripts/) | Core artifact import and verification |
| [`bridge-tests/`](bridge-tests/) | Rust bridge tests with optional native core coverage |

## Legacy workspace

The multi-crate Rust workspace, gateway channels, CLI/TUI, and their configuration and build instructions are documented in the [legacy English README](https://github.com/ProjectViVy/agent-diva/blob/dev/README.md) and [legacy Chinese README](https://github.com/ProjectViVy/agent-diva/blob/dev/README.zh-CN.md) on `dev`. See the [legacy workspace’s MIT license](https://github.com/ProjectViVy/agent-diva/blob/dev/LICENSE).
