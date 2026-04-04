# Justfile for Agent Diva

# CI/Linux/macOS 使用 bash；Windows 本地开发使用 PowerShell（避免在 ubuntu-latest 上找不到 powershell.exe）
set shell := ["bash", "-cu"]
set windows-shell := ["powershell.exe", "-NoProfile", "-c"]

# Default recipe - show help
default:
    @just --list

# Start both Gateway and GUI
start:
    Start-Process -FilePath "cargo" -ArgumentList "run --bin agent-diva -- gateway"
    cd agent-diva-gui; npm run tauri dev

# Build all crates
build:
    cargo build --all

# Build in release mode
build-release:
    cargo build --all --release

# Run tests
test:
    cargo test --all

# Run clippy check
check:
    cargo clippy --all -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Generate documentation
doc:
    cargo doc --all --no-deps

# Generate and open documentation
doc-open:
    cargo doc --all --no-deps --open

# Run the CLI
run *ARGS:
    cargo run --package agent-diva-cli -- {{ARGS}}

# Run the migration tool
migrate *ARGS:
    cargo run --package agent-diva-migration -- {{ARGS}}

# Clean build artifacts
clean:
    cargo clean

# Run all checks (CI pipeline)
ci: fmt-check check test
    @echo "All checks passed!"

# Install locally
install:
    cargo install --path agent-diva-cli

# Update dependencies
update:
    cargo update

# Audit dependencies
audit:
    cargo audit

# Run benchmarks
bench:
    cargo bench --all

# Linux x86_64 zip (Windows host: requires Docker + `cross`; CI/Linux: use scripts/package-linux.sh)
package-linux: cross-linux-x86_64
    New-Item -ItemType Directory -Force -Path "dist\linux"
    Copy-Item "target\x86_64-unknown-linux-gnu\release\agent-diva" "dist\linux\agent-diva" -Force
    Compress-Archive -Path "dist\linux\agent-diva" -DestinationPath "dist\agent-diva-linux-x86_64.zip" -Force
    Write-Host "Package created: dist\agent-diva-linux-x86_64.zip"

# Build deb package (requires cargo-deb on Linux)
build-deb:
    cargo deb -p agent-diva-cli

# Build all release packages (Linux only)
build-all-packages:
    cargo build --release --package agent-diva-cli
    cargo deb -p agent-diva-cli
    cargo generate-rpm -p agent-diva-cli 2>$null || Write-Host "RPM generation skipped (cargo-generate-rpm not installed)"
    Write-Host "All packages built"

# Install cargo-deb tool
install-cargo-deb:
    cargo install cargo-deb

# Install cross for cross-compilation
install-cross:
    cargo install cross

# Cross-compile for Linux x86_64 (requires Docker)
cross-linux-x86_64:
    cross build --release --target x86_64-unknown-linux-gnu -p agent-diva-cli

# Cross-compile for Linux ARM64 (requires Docker)
cross-linux-arm64:
    cross build --release --target aarch64-unknown-linux-gnu -p agent-diva-cli

# Trigger GitHub Actions CI (desktop GUI 三平台构建；打 v*.*.* tag 时同一次运行会发 Release)
trigger-build:
    gh workflow run CI

# Windows GUI 安装包（NSIS + MSI）：一键 cargo release、bundle:prepare、tauri build
package-windows-gui:
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/package-windows-gui.ps1


# ===== macOS 打包命令 (需要在 macOS 上运行) =====
# 在 macOS 上使用: ./scripts/package-macos.sh

# Build macOS universal binary (macOS only)
build-macos-universal:
    #!/bin/bash
    rustup target add x86_64-apple-darwin aarch64-apple-darwin
    cargo build --release --package agent-diva-cli --target x86_64-apple-darwin
    cargo build --release --package agent-diva-cli --target aarch64-apple-darwin
    mkdir -p target/universal/release
    lipo -create target/x86_64-apple-darwin/release/agent-diva target/aarch64-apple-darwin/release/agent-diva -output target/universal/release/agent-diva
    echo "Universal binary created: target/universal/release/agent-diva"

# Create macOS DMG (macOS only, requires create-dmg)
build-macos-dmg:
    #!/bin/bash
    if ! command -v create-dmg &> /dev/null; then
        echo "Installing create-dmg..."
        brew install create-dmg
    fi
    ./scripts/package-macos.sh
