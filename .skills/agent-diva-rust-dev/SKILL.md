---
name: agent-diva-rust-dev
description: Strengthen Rust development workflows inside the Agent Diva Rust workspace. Use this when working on Rust crates (Cargo.toml, .rs files) in this repo and you want help with: (1) Designing/refactoring module structures, (2) Implementing and testing async/concurrent logic, (3) Improving performance and memory safety, (4) Enforcing strict error handling and safety constraints, (5) Aligning with this repository's Rust coding conventions and toolchains.
---

# Agent Diva Rust Dev

## Overview

This Skill focuses entirely on **writing production-ready Rust code within the `agent-diva` Rust workspace**. From module organization, trait design, and error handling to Tokio async, performance optimization, memory safety, and testing practices—everything is centered around the strict conventions and toolchains of this repository. The Agent must output code meeting the standards of a "Senior Rust Engineer."

## Applicable Scenarios

Prioritize using this Skill when:

- Modifying `.rs` files or `Cargo.toml` in any `agent-diva-*` crate.
- Implementing traits for new Providers, Channels, or Tools.
- Adding/refactoring tests, async logic, or performance-critical paths for core modules.
- Aligning overall with the Rust coding conventions specified in `AGENTS.md`/`CLAUDE.md` of this repository.

## Task-Based Perspective

### Task 1: Creating or Extending Rust Modules/Crates (Architecture & Visibility)

1. **Determine Crate Ownership:**
   - Provider-related → `agent-diva-providers`
   - Channel/Platform adaptation-related → `agent-diva-channels`
   - Agent loop / Context-related → `agent-diva-agent`
   - Common types, configurations, event bus → `agent-diva-core`
   - CLI behavior → `agent-diva-cli`
2. **Dependency & Version Management:**
   - New dependencies MUST be added to the workspace root `Cargo.toml` under `[workspace.dependencies]` with explicit version numbers.
   - Individual crates should only reference them via `{ workspace = true }`. Declaring versions in child crates is strictly prohibited.
3. **Strict Visibility & Encapsulation:**
   - Keep items private by default. Prefer `pub(crate)` or `pub(super)` to scope visibility. Only use `pub` for interfaces that genuinely need external exposure.
   - Use the **Newtype Pattern** to wrap primitive types (e.g., `pub struct AgentId(String);`) to prevent misuse and provide compile-time type safety.
4. **Naming & Structure:**
   - Use `snake_case` for modules/files and `PascalCase` for types and traits.
   - Adhere to the Rust API Guidelines (e.g., C-BUILDER, C-GETTER conventions).

### Task 2: Implementing Provider / Channel / Tool Traits

1. **Provider (in `agent-diva-providers`):**
   - Adhere to the **provider-model-id-safety** rule: Native Providers must retain the original model ID without automatically appending prefixes.
   - HTTP clients MUST be reused. Recreating `reqwest::Client` for every request is strictly prohibited.
   - Prefer zero-cost abstractions for request/response structs (e.g., using `&'a str` with `#[serde(borrow)]` during deserialization to minimize memory allocations).
2. **ChannelHandler (in `agent-diva-channels`):**
   - Focus heavily on: Message send/receive flows, retry strategies, and rate limit handling (integrating with `tokio-util` or rate-limiting crates).
3. **Tool Implementation (in `agent-diva-tools`):**
   - Input/Output structs must derive `Serialize, Deserialize, Debug, Clone`.
   - Sensitive information (e.g., API Keys) MUST be wrapped using `secrecy::SecretString` to prevent accidental logging.

### Task 3: Async & Concurrency Constraints (Tokio & Shared State)

When dealing with I/O, scheduled tasks, or high-concurrency scenarios, you MUST obey the following ironclad rules:

- **No Blocking:** Using `std::sync::Mutex`, `std::thread::sleep`, or blocking I/O operations inside an `async` context is absolutely forbidden.
- **Lock Granularity:** When state must be shared, evaluate if shared locks can be eliminated via the Actor model (MPSC channels). If locking is unavoidable, prefer `tokio::sync::RwLock` and keep critical sections extremely short.
- **Channel Capacity:** When using `tokio::sync::mpsc`, a reasonable capacity MUST be explicitly specified. **Using unbounded queues (`unbounded_channel`) is strictly prohibited** to prevent OOM errors.
- **Preventing Task Hangs:**
  - When spawning background tasks with `tokio::spawn`, graceful shutdown MUST be considered (e.g., using `tokio_util::sync::CancellationToken`).
  - All cross-network I/O Futures MUST be wrapped in `tokio::time::timeout`.

### Task 4: Error Handling & Logging (Zero-Tolerance Policy)

- **No Panics:** Production code **absolutely forbids** the use of `.unwrap()`, `.expect()`, or `panic!()`. All operations that can fail must return a `Result`. If a failure is logically impossible, you must document it with `// SAFETY: ...` and use `unreachable!()` if necessary.
- **Library-Level Errors:** Base crates like `agent-diva-core` must use `thiserror` to define precise error `enum`s and implement `#[error("...")]` for semantic context.
- **Application-Level Errors:** CLI or Agent top-level callers should use `anyhow::{Result, Context}` to wrap errors. When an error occurs, context variables MUST be attached via `.context("failed to do X")`.
- **Observability (Tracing):**
  - Do not use `println!`. Standardize on `tracing` macros (`info!`, `debug!`, `warn!`, `error!`, `instrument`).
  - Add the `#[tracing::instrument(skip(self, sensitive_data), err)]` macro to critical async functions to automatically trace entry/exit and errors.

### Task 5: Performance, Memory, and Idiomatic Rust

- **Cloning & References:** Avoid mindless `.clone()` calls to escape lifetime issues. If cloning is expensive (e.g., large `Vec` or `String`), consider passing a borrow `&T` or using `Arc<T>`.
- **Pre-allocating Memory:** When collection sizes are known, you MUST use `Vec::with_capacity()` or `HashMap::with_capacity()`.
- **Unsafe Zone:** Unless dealing with highly performance-critical FFI or low-level parsing code, **the use of `unsafe` is strictly forbidden**. If it must be used, it MUST include a `# SAFETY:` doc block explaining why it is memory-safe.
- **Leverage Standard Library Traits:** Actively implement and use standard traits like `From` / `TryFrom` (for type conversions), `Default`, `AsRef`, etc., to make APIs elegant.

### Task 6: Testing & Validation

- **Unit Tests:** Use `#[cfg(test)]`, and `#[tokio::test]` for async tests. Verify the happy path and **all error boundary paths**.
- **Mocking:** When testing logic involving external APIs, trait usage is encouraged alongside `mockall` to automatically generate Mock objects.
- **Documentation Constraints:** All public (`pub`) modules, structs, traits, and functions MUST have `///` Rustdoc comments. Complex APIs MUST include `/// # Examples` code blocks in their documentation.
- **Workspace Validation:** After generating code, it must pass (conceptually or practically):
  - `cargo clippy --all-targets --all-features -- -D warnings -D clippy::unwrap_used -D clippy::expect_used`
  - `cargo fmt --all`

Prefer the in-repo **`agent-diva-workspace-validate`** skill for the canonical `just` recipes (`just ci`, `just check`, etc.).

## Workflow-Based Perspective

When planning significant changes to the Rust code, follow this sequence:

1. **Read Relevant Docs**: First, review `AGENTS.md` and `CLAUDE.md` in the root directory to understand the project architecture and conventions.
2. **Locate the Blast Radius**: Determine which crates, modules, or feature flags are affected by the change.
3. **Design API Boundaries & Types**: Define strict `struct`/`enum` and `trait` interfaces first (thinking through ownership and error types), then fill in the implementation details.
4. **Add Tests & Docs**: Write unit tests for new/modified logic, write comprehensive Rustdocs, and add a small integration test if necessary.
5. **Execute Clippy-Driven Development**: Fix all potential Clippy warnings, ensuring zero `unwrap` instances exist.

## Quick Usage Examples

Typical invocation scenarios include, but are not limited to:

- "Add a new LLM Provider implementation in `agent-diva-providers`. Help me design a zero-copy deserialization struct, a `thiserror` tree, and async requests with timeouts."
- "Refactor the event bus in `agent-diva-core`. Replace the current `Mutex` with MPSC and `CancellationToken` to eliminate potential deadlocks, and provide a testing strategy."
- "Review this Agent execution loop code, identify areas that might block the Tokio executor, and rewrite it into a strictly non-blocking state machine."

## Related skills

- **`agent-diva-extend-integrations`**, **`agent-diva-core-data-flow`**, **`agent-diva-manager-gateway`**: domain-specific checklists in `.cursor/skills/`.

## Resources

This Skill currently relies primarily on text-based guidance. If needed later, you can populate:

- `references/`:
  - Cheat sheets for common traits/modules in this repository.
  - Example patterns for common Provider/Channel implementations.
- `scripts/`:
  - Wrappers for common `cargo`/`just` commands or validation scripts.
- `assets/`:
  - Rust module templates, unified `clippy.toml` configuration examples, etc.
