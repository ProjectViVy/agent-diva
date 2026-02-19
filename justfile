# Justfile for nanobot

# Default recipe - show help
default:
    @just --list

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
    cargo run --package nanobot-cli -- {{ARGS}}

# Run the migration tool
migrate *ARGS:
    cargo run --package nanobot-migration -- {{ARGS}}

# Clean build artifacts
clean:
    cargo clean

# Run all checks (CI pipeline)
ci: fmt-check check test
    @echo "All checks passed!"

# Install locally
install:
    cargo install --path nanobot-cli

# Update dependencies
update:
    cargo update

# Audit dependencies
audit:
    cargo audit

# Run benchmarks
bench:
    cargo bench --all
