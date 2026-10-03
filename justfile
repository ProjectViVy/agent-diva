# Justfile for Agent Diva — post wire-cut (DN-W).
# The legacy Rust workspace and Tauri backend were removed on this branch;
# only GUI recipes remain. The new product build contract arrives in DN-8
# (see docs/plans/diva-next/index.md).

set shell := ["bash", "-cu"]
set windows-shell := ["powershell.exe", "-NoProfile", "-c"]

# Default recipe - show help
default:
    @just --list

# Install GUI dependencies (pnpm, project-pinned version)
gui-install:
    pnpm --dir agent-diva-gui install --frozen-lockfile

# Run GUI unit/component tests
gui-test:
    pnpm --dir agent-diva-gui test

# Build the GUI (vue-tsc + Vite)
gui-build:
    pnpm --dir agent-diva-gui build

# Run the GUI dev server (browser mode)
gui-dev:
    pnpm --dir agent-diva-gui dev

# --- DN-5 thin shell (Tauri v2 + vivy-bridge) ---

# Run the pure-Rust bridge FFI tests (no webkit needed; CI-safe)
shell-bridge-test:
    cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml -p vivy-bridge

# Full shell test suite (requires webkit2gtk-4.1 dev libs on Linux)
shell-test:
    cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml --workspace

# Clippy gate for the shell workspace
shell-clippy:
    cargo clippy --manifest-path agent-diva-gui/src-tauri/Cargo.toml --all-targets -- -D warnings

# Stage a sealed DN-L artifact dir into src-tauri/vivy-runtime for bundling
shell-stage-runtime ARTIFACT_DIR:
    rsync -a --delete {{ARTIFACT_DIR}}/ agent-diva-gui/src-tauri/vivy-runtime/
    @echo "staged: {{ARTIFACT_DIR}} -> src-tauri/vivy-runtime"

# Native package build (frontend build + cargo release + bundle)
tauri-build: gui-build
    pnpm --dir agent-diva-gui tauri build

# All current checks
ci: gui-test gui-build shell-bridge-test
    @echo "All checks passed!"
