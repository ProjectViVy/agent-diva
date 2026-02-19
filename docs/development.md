# Development Guide

This guide covers development practices and workflows for agent-diva.

## Getting Started

### Setting Up Your Environment

1. **Install Rust** (1.70+):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install required tools**:
   ```bash
   # Just (command runner)
   cargo install just

   # cargo-deny (license/security checking)
   cargo install cargo-deny

   # cargo-tarpaulin (code coverage)
   cargo install cargo-tarpaulin
   ```

3. **Clone and build**:
   ```bash
   git clone https://github.com/HKUDS/agent-diva.git
   cd Agent Diva/agent-diva
   cargo build --all
   ```

### IDE Setup

#### VS Code

Recommended extensions:
- rust-analyzer
- Even Better TOML
- CodeLLDB (debugging)
- Error Lens

#### RustRover / IntelliJ

The Rust plugin provides excellent support for:
- Code completion
- Refactoring
- Debugging
- Cargo integration

## Development Workflow

### Making Changes

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes** with tests

3. **Run checks**:
   ```bash
   just ci
   ```

4. **Commit with a clear message**:
   ```bash
   git commit -m "Add feature X

   - Implement core functionality
   - Add tests
   - Update documentation"
   ```

### Code Review Checklist

Before submitting a PR:

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] New code has tests
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated (if applicable)
- [ ] Commit messages are clear

## Coding Standards

### Rust Style Guidelines

We follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and:

- Use `snake_case` for functions and variables
- Use `PascalCase` for types and traits
- Use `SCREAMING_SNAKE_CASE` for constants
- Use `#[must_use]` for important return values
- Document all public APIs with `///`

### Error Handling

```rust
// Use Result for fallible operations
pub fn load_config() -> Result<Config, Error> {
    // ...
}

// Use thiserror for library errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid configuration: {0}")]
    Config(String),
}

// Use anyhow for application errors
fn main() -> anyhow::Result<()> {
    let config = load_config()?;
    Ok(())
}
```

### Async Patterns

```rust
// Prefer async/await over manual Futures
pub async fn process_message(&self, msg: Message) -> Result<Response> {
    let data = self.fetch_data().await?;
    self.transform(data).await
}

// Use channels for communication
let (tx, rx) = mpsc::unbounded_channel();

// Spawn tasks for concurrent operations
tokio::spawn(async move {
    while let Some(msg) = rx.recv().await {
        process(msg).await;
    }
});
```

### Testing

```rust
// Unit tests in the same file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_functionality() {
        let result = my_function(42);
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_functionality() {
        let result = my_async_function().await;
        assert!(result.is_ok());
    }
}
```

## Debugging

### Logging

Use `tracing` for structured logging:

```rust
use tracing::{info, debug, error, warn};

info!(user_id = %user.id, "Processing message");
debug!(?config, "Loaded configuration");
warn!(attempt = retry_count, "Retrying request");
error!(error = %e, "Failed to process message");
```

Set log level via environment:
```bash
RUST_LOG=debug cargo run
```

### Debugging with VS Code

Create `.vscode/launch.json`:

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug agent-diva",
            "cargo": {
                "args": ["build", "--package", "agent-diva-cli"],
                "filter": {
                    "name": "agent-diva",
                    "kind": "bin"
                }
            },
            "args": ["status"],
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

## Performance Profiling

### Using cargo-flamegraph

```bash
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --package agent-diva-cli --bin Agent Diva

# View flamegraph.svg in browser
```

### Using perf on Linux

```bash
# Build with debug symbols
cargo build --release --package agent-diva-cli

# Profile
perf record -g ./target/release/agent-diva status
perf report
```

## Common Tasks

### Adding a New Dependency

1. Add to workspace `Cargo.toml`:
   ```toml
   [workspace.dependencies]
   new-crate = "1.0"
   ```

2. Use in crate `Cargo.toml`:
   ```toml
   [dependencies]
   new-crate = { workspace = true }
   ```

3. Run `cargo check` to verify

### Updating Dependencies

```bash
# Update all dependencies
cargo update

# Update specific crate
cargo update --package serde

# Check for outdated crates
cargo install cargo-outdated
cargo outdated
```

### Running Specific Tests

```bash
# Run specific test
cargo test test_name

# Run tests in specific crate
cargo test --package agent-diva-core

# Run tests matching pattern
cargo test message_bus

# Run with output
cargo test -- --nocapture
```

### Code Coverage

```bash
# Generate coverage report
cargo tarpaulin --all --out html

# Open coverage report
open tarpaulin-report.html
```

## Troubleshooting

### Build Issues

**Problem**: Build fails with linking errors
**Solution**: 
```bash
# Clean and rebuild
cargo clean
cargo build
```

**Problem**: Dependency conflicts
**Solution**:
```bash
# Check dependency tree
cargo tree

# Update lockfile
cargo update
```

### Test Issues

**Problem**: Tests fail intermittently
**Solution**: Check for race conditions, use proper synchronization

**Problem**: Async tests hang
**Solution**: Ensure all spawned tasks complete, use timeouts

### IDE Issues

**Problem**: rust-analyzer shows errors but code compiles
**Solution**: Restart rust-analyzer or VS Code

**Problem**: Breakpoints not hit
**Solution**: Build with debug symbols: `cargo build`

## Release Process

1. **Update version** in workspace `Cargo.toml`
2. **Update CHANGELOG.md**
3. **Create git tag**:
   ```bash
   git tag v0.x.x
   git push origin v0.x.x
   ```
4. **CI automatically**:
   - Builds binaries for all platforms
   - Creates GitHub release
   - Publishes to crates.io

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Tokio Documentation](https://tokio.rs/)
- [Thiserror Documentation](https://docs.rs/thiserror/)
- [Anyhow Documentation](https://docs.rs/anyhow/)
