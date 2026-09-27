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

# All current checks
ci: gui-test gui-build
    @echo "All checks passed!"
