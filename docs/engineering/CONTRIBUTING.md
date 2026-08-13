# Contributing to Agent Diva

Thank you for your interest in Contributing to Agent Diva! This document provides guidelines and information for contributors.

## Code of Conduct

This project adheres to a code of conduct. By participating, you are expected to uphold this code:

- Be respectful and inclusive
- Welcome newcomers
- Focus on constructive feedback
- Respect different viewpoints and experiences

## How to Contribute

### Reporting Bugs

If you find a bug, please open an issue with the following information:

- Clear description of the bug
- Steps to reproduce
- Expected behavior
- Actual behavior
- Environment details (OS, Rust version, etc.)
- Any relevant logs or error messages

### Suggesting Features

Feature suggestions are welcome! Please open an issue with:

- Clear description of the feature
- Use case and motivation
- Possible implementation approach (if you have ideas)

### Pull Requests

1. Fork the repository
2. Create a new branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run the tests (`cargo test --all`)
5. Run clippy (`cargo clippy --all -- -D warnings`)
6. Format your code (`cargo fmt --all`)
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- Just (optional but recommended): `cargo install just`

### Building

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/Agent Diva.git
cd Agent Diva/agent-diva

# Build all crates
cargo build --all

# Run tests
cargo test --all
```

### Code Style

We use the following tools to maintain code quality:

- **rustfmt**: Code formatting
- **clippy**: Linting
- **cargo-deny**: License and security auditing

Run all checks with:

```bash
just ci
```

Or manually:

```bash
cargo fmt --all -- --check
cargo clippy --all -- -D warnings
cargo test --all
```

### Commit Messages

Please follow these guidelines for commit messages:

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters or less
- Reference issues and pull requests liberally after the first line

Example:

```
Add support for Discord threads

- Implement thread creation and management
- Add configuration option for thread policy
- Update documentation

Fixes #123
```

## Project Structure

Understanding the project structure will help you contribute effectively:

### Crate Organization

- **agent-diva-core**: Foundational types and traits used by all other crates
- **agent-diva-agent**: Agent loop, context building, and skill loading
- **agent-diva-providers**: LLM provider abstractions and implementations
- **agent-diva-channels**: Chat platform integrations
- **agent-diva-tools**: Built-in tool implementations
- **agent-diva-cli**: Command-line interface
- **agent-diva-migration**: Migration tool from Python version

### Adding a New Channel

To add support for a new chat platform:

1. Create a new module in `agent-diva-channels/src/`
2. Implement the `ChannelHandler` trait
3. Add configuration to `agent-diva-core/src/config/schema.rs`
4. Add CLI commands in `agent-diva-cli/src/main.rs`
5. Write tests
6. Update documentation

### Adding a New Tool

To add a new tool:

1. Create a new module in `agent-diva-tools/src/`
2. Implement the `Tool` trait
3. Register the tool in the `ToolRegistry`
4. Write tests
5. Update documentation

### Adding a New Provider

To add a new LLM provider:

1. Create a new module in `agent-diva-providers/src/`
2. Implement the `LLMProvider` trait
3. Add configuration to `agent-diva-core/src/config/schema.rs`
4. Register the provider in the provider registry
5. Write tests
6. Update documentation

## Testing

### Unit Tests

Write unit tests for individual functions and modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        assert_eq!(my_function(2), 4);
    }
}
```

### Integration Tests

Integration tests are in the `tests/` directory of each crate.

### Running Tests

```bash
# Run all tests
cargo test --all

# Run tests for a specific crate
cargo test --package agent-diva-core

# Run with output
cargo test --all -- --nocapture
```

## Documentation

- Use `///` for public API documentation
- Use `//!` for module-level documentation
- Include examples in documentation where helpful
- Build and check documentation with `cargo doc --all --no-deps`

## Release Process

1. Update version in workspace `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create a git tag (`git tag v0.x.x`)
4. Push the tag (`git push origin v0.x.x`)
5. The CI will automatically create a release

## Getting Help

If you need help or have questions:

- Open an issue on GitHub
- Check existing documentation
- Look at existing code for examples

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
