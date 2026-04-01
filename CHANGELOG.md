# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Documentation
- **V1.0.0 发布门禁（Story 6.7）：** 产品向 **1.0.0** 双轨 P0 机械核对表定稿于仓库 `../_bmad-output/planning-artifacts/release-checklist-v1.0.0.md`（与 `../_bmad-output/planning-artifacts/prd.md` 互链）。**发版前**须在该表中完成勾选并对齐此处版本记录与根 `Cargo.toml` `workspace.package.version`。

### Added
- **Swarm / FR2（Story 1.5）:** `agent-diva-swarm` `process_events` — versioned whitelist DTOs (`ProcessEventV0`, `swarm_phase_changed` / `tool_call_started` / `tool_call_finished`), `ProcessEventPipeline` with cortex gate and throttle (default 100ms / batch 32; tool milestones flush immediately). `AgentLoop::with_process_event_pipeline` wires emits on iteration and tool boundaries; docs in `agent-diva-swarm/docs/process-events-v0.md` and `PROCESS_EVENTS_CORTEX_OFF.md`; contract cross-link in root `docs/swarm-cortex-contract-v0.md`.
- **Swarm / GUI（FR13–FR14）:** Documented cortex sync contract v0 in `docs/swarm-cortex-contract-v0.md`; Tauri commands `get_cortex_state`, `set_cortex_enabled`, `toggle_cortex` delegate to `agent-diva-swarm` `CortexRuntime` with `cortex_toggled` event payload (`schemaVersion` = v0). `CortexState` JSON uses camelCase wire fields; `CortexSyncDto` type alias added for the stable DTO name.
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
- **Matrix Channel**: Added Matrix channel support with long-polling sync, text/media delivery, deduplication, and configurable media limits.
- **Workspace Bootstrap**: Added automatic workspace template sync (`MEMORY.md`, `HISTORY.md`, `PROFILE.md`, `TASK.md`) on onboard and runtime entry points.
- **Thinking Config**: Added `agents.defaults.reasoning_effort` (low/medium/high) and provider passthrough for thinking-capable models.

### Fixed
- **GUI / FR13（code review 1.3）:** `docs/swarm-cortex-contract-v0.md` 补全 `get_neuro_overview_snapshot` 与 `NeuroOverviewSnapshotV0` 白名单；`cortex_toggled` 的 `emit` 失败时记录 `tracing::warn`；皮层同步钩相关测试使用 `serial_test` 避免 `AGENT_DIVA_TEST_CORTEX_SYNC_FAIL` 并行竞态。
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
- **Session Hardening**: Prevented error-finish and empty assistant messages from polluting persisted session history.
- **WhatsApp**: Added inbound deduplication for reconnect replay protection.
- **DingTalk**: Added outbound media support (URL/local file upload and send).
- **Tools**: Hardened Windows absolute-path detection in shell workspace guard.
- **Provider Compatibility**: Normalized assistant tool-call messages to use `content: null` for stricter OpenAI-compatible providers (e.g. Mistral-native endpoints).

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
