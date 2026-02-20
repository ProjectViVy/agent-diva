# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **GUI**: Enhanced `agent-diva-gui` with complete model and channel configuration capabilities.
- **Manager**: Added `agent-diva-manager` to simplify configuration management.
- **Reasoning Streaming**: Added real-time streaming of reasoning content (`ReasoningDelta`) from compatible models (e.g., DeepSeek Reasoner) to the GUI.
- **Reasoning UI**: Implemented a collapsible "Thinking Process" section in chat bubbles with state-aware animations (pulsing when thinking, static when done).
- **Gateway**: Restored "Gateway One-Click Start" mode. The `agent-diva-cli` now starts all core components (Agent, Manager, Server) in a single unified process.
- **Manager**: Implemented `Manager` as a sidecar control plane that runs in parallel with the main agent loop.
- **Dynamic Provider**: Added `DynamicProvider` to support runtime hot-swapping of LLM providers without restarting the gateway.
- **Reasoning Content**: Added support for displaying reasoning content (`<think>`) from reasoning models (e.g., DeepSeek Reasoner) in both CLI and GUI.
- **Markdown Support**: Integrated `markdown-it` and `highlight.js` to render Markdown content in chat messages with syntax highlighting (GitHub Dark theme).
- **Browser Compatibility**: Added mock implementations for Tauri APIs to support development and testing in standard web browsers.
- **Loading Indicators**: Added a loading animation within the message bubble to indicate when the agent is "thinking" or generating a response.

### Fixed
- **Dependency**: Resolved duplicate import errors for `LiteLLMClient` and `ProviderRegistry`.
- **Concurrency**: Fixed issue where the Manager would take ownership of the AgentLoop, preventing it from running.
- **Manager**: Fixed "channel closed" error during configuration update by implementing **Hot Reloading** for LLM providers instead of restarting the gateway.
- **Agent Loop**: Fixed stale model usage where the agent loop would continue using the initial model after a configuration update. Now it dynamically fetches the latest model from the provider on each request.
- **GUI**: Fixed provider display issue where suppliers were not showing correctly and API key placeholder was "undefined".
- **Manager**: Fixed potential panic in manager loop by correctly handling channel closure.
- **Agent Loop**: Fixed compilation errors in `agent-diva-agent` by correctly handling `reasoning_content` in `OutboundMessage` and fixing `unwrap_or_default` misuse on `String` type.
- **CLI**: Fixed non-exhaustive patterns error in `agent-diva-cli` by adding support for `TimelineKind::Thinking` in the TUI timeline renderer.
- **Core**: Added `reasoning_content` field to `OutboundMessage` struct in `agent-diva-core` to support passing reasoning data through the message bus.
- **Config**: Fixed whitespace handling in API base URL configuration to prevent connection errors when copying URLs with spaces.
- **GUI**: Fixed `deepseek-reasoner` model usage where reasoning content was not being displayed in the chat interface.
- **Agent Loop**: Fixed `reasoning_content` field missing in `Message` struct causing API errors with reasoning models (e.g., DeepSeek Reasoner) during tool calls.
- **Channels**: Fixed `ChannelHandler` trait definition and implementation to support `set_inbound_sender` method across all channel integrations (Telegram, Discord, Feishu, WhatsApp, DingTalk, Email, Slack, QQ).
- **QQ Channel**:
    - Fixed authentication protocol to use `QQBot` prefix and added `X-Union-Appid` header.
    - Implemented automatic token refresh and expiration tracking.
    - Fixed "Active push not allowed" error by correctly handling `message_id` for passive replies.
    - Enabled `native-tls` for WebSocket connections to support `wss://` gateway.
    - Updated event intents to correctly receive public and direct messages.
- **GUI**: Added missing `App Secret` field for QQ channel configuration.

### Changed
- **Gateway**: Major refactor of channel logic, making `gateway` the unified entry point.
- **Message Data Model**: Updated `Message` interface in GUI to support `reasoning` and `isThinking` states.
- **Tool Call UI**: Refactored the tool call display to use a single, dynamic card component. The card now updates its status (Running -> Success/Error) in-place, reducing clutter and improving readability.
- **Input Bar Layout**: Optimized the chat input area.
    - Aligned the text input, send button, and clear button horizontally using Flexbox.
    - Added auto-resizing capability to the textarea.
    - Enhanced visual styling with rounded corners, shadows, and focus states using Tailwind CSS.
- **Message Rendering**: Improved the rendering logic to prevent empty message bubbles from appearing before content is received.

### Removed
- **Redundant Loading Bar**: Removed the fixed loading indicator at the bottom of the chat view in favor of the in-bubble loading state.
