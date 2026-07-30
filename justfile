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

# Start Gateway only
diva-gate:
    cargo run --package agent-diva-cli -- gateway run

# Build all crates
build:
    cargo build --all

# Build in release mode
build-release:
    cargo build --all --release

# Run tests
test:
    cargo test --all

# Run real-provider E2E tests explicitly from the root workspace
e2e-test:
    cargo test -p agent-diva-e2e -- --nocapture

# Run focused Memory-provider assembly and failure regressions
memory-provider-check:
    cargo check -p agent-diva-agent --no-default-features
    cargo test -p agent-diva-agent test_build_agent_tools_reuses_custom_tools_with_cron
    cargo test -p agent-diva-agent test_register_default_tools_preserves_custom_tools_with_cron
    cargo test -p agent-diva-agent test_agent_loop_prefetch_failure_continues_without_recall_injection
    cargo test -p agent-diva-agent test_agent_loop_consolidation_sync_failure_keeps_main_response

# Prove that Embedded Laputa is the sole production Memory runtime.
laputa-clean-break-check:
    python scripts/ci/check_laputa_clean_break.py

# Deterministic crash-window, backup, replay, and rollback drills for E7.
e7-recovery-drills:
    cargo test -p agent-diva-laputa --test apply
    cargo test -p agent-diva-laputa --test governed_apply
    cargo test -p agent-diva-laputa --test migration
    cargo test -p agent-diva-laputa --test typed_store canonical_open_migrates_only_workspace_identity_and_keeps_verified_backup -- --exact
    cargo test -p agent-diva-migration workspace_identity::tests::identity_dry_run_apply_and_rollback_are_explicit_and_reversible -- --exact
    cargo test -p agent-diva-manager --lib handlers::laputa::recovery_tests::prepared_journal_recovers_commit_then_consumes_receipt -- --exact

# Provider-free proof of the complete product Memory lifecycle.
e7-vertical-e2e:
    cargo test -p agent-diva-manager --lib server::tests::vertical_autodream_typed_memory_recall_feedback_and_rollback_closes -- --exact

# Automated desktop gates; this intentionally does not claim real-desktop G2D+.
gui-automated-check:
    cd agent-diva-gui; npm test
    cd agent-diva-gui; npm run build
    cargo check --manifest-path agent-diva-gui/src-tauri/Cargo.toml

# Final automated E7 candidate gate. Manual desktop acceptance remains separate.
e7-automated-release-gate: fmt-check check test health-benchmark-check feature-gate-check laputa-clean-break-check e7-recovery-drills e7-vertical-e2e gui-automated-check
    @echo "E7 automated release gate passed; G2D+ real-desktop acceptance remains deferred."

# Run clippy check
check:
    cargo clippy --all -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Targeted `/api/health` benchmark-style validation used by CI/review closure
health-benchmark-check:
    cargo test -p agent-diva-manager health_benchmark_ci_gate_stays_within_budget -- --nocapture

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

# Verify each non-default feature gate compiles individually
feature-gate-check:
    python scripts/feature-gate-check.py

# Run all checks (CI pipeline)
ci: fmt-check check test health-benchmark-check feature-gate-check laputa-clean-break-check
    @echo "All checks passed!"

# Epic 6 targeted governance proof checks without starting the GUI
epic6-proof-check:
    cargo test -p agent-diva-laputa --test authority_boundaries
    cargo test -p agent-diva-laputa --test storage
    cargo test -p agent-diva-laputa --test migration
    cargo test -p agent-diva-laputa --test apply

# Story 6.5 release gate for EVO-DIVA governance readiness
epic6-release-gate:
    cargo test -p agent-diva-laputa --test authority_boundaries
    cargo test -p agent-diva-laputa --test direct_write_guard
    cargo test -p agent-diva-laputa --test governance_proof_loop
    cargo test -p agent-diva-laputa --test service
    cargo test -p agent-diva-autodream --test service
    cargo check -p agent-diva-gui

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

# 不要改！手动测试用的。
build-windows:
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
