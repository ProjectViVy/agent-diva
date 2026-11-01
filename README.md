# Agent Diva

A fresh desktop foundation built with Tauri 2, Vue 3, TypeScript and Vite.
The previous Go implementation is preserved on the `go-archive` branch.

## Development

Install Node.js 24 LTS, npm, stable Rust, and the Tauri platform prerequisites:
https://v2.tauri.app/start/prerequisites/

On Windows, install the MSVC C++ build tools and WebView2 runtime.

```sh
npm ci
npm run tauri dev
```

For a browser-only preview, run `npm run dev`. Native host calls are available
only inside the Tauri desktop application.

## Validation and packaging

```sh
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm run tauri build -- --debug --no-bundle
npm run tauri build
```

The final command creates a release build and platform installers. CI validates
the frontend and an unbundled Windows debug build; macOS/Linux and release
installers require separate verification.

## Layout and scope

- `src/`: Vue application and native host connection state.
- `src-tauri/`: Rust desktop entry point, commands, capabilities and packaging.
- `.github/workflows/ci.yml`: Windows build validation.

The welcome screen invokes the read-only Rust `app_info` command and reports
connection success or failure. Browser preview reports that no native host is
available. No model provider, credentials, chat runtime, shell execution,
session persistence or legacy Go code is included. Default icons are the Tauri
template placeholders.

This repository is independent of the vivy-test experiment. It does not depend
on that checkout or its data.
