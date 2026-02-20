# Justfile for nanobot

set shell := ["powershell.exe", "-c"]

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
