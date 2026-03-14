# Architecture Overview

This document provides an overview of the agent-diva architecture.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI (agent-diva-cli)                     │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌──────────────┐    ┌─────────────────┐    ┌──────────────┐
│   Channels   │    │   Agent Loop    │    │    Tools     │
│              │    │                 │    │              │
│ • Telegram   │◄──►│ • Context       │◄──►│ • Filesystem │
│ • Discord    │    │ • Skills        │    │ • Shell      │
│ • Slack      │    │ • Subagents     │    │ • Web        │
│ • WhatsApp   │    │                 │    │ • Message    │
│ • Feishu     │    │                 │    │ • Spawn      │
│ • DingTalk   │    │                 │    │ • Cron       │
│ • Email      │    │                 │    │              │
│ • QQ         │    │                 │    │              │
└──────────────┘    └─────────────────┘    └──────────────┘
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     agent-diva-core                             │
│                                                              │
│  • Message Bus    • Configuration    • Session Management    │
│  • Memory System  • Error Types      • Utilities             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   agent-diva-providers                          │
│                                                              │
│  • OpenRouter  • Anthropic  • OpenAI  • DeepSeek  • Groq    │
│  • Gemini      • Zhipu      • DashScope  • Moonshot         │
│  • vLLM (local)  • AiHubMix                                  │
└─────────────────────────────────────────────────────────────┘
```

## Crate Responsibilities

### agent-diva-core

The foundation of the system. Provides:

- **Message Bus**: Dual-queue system for decoupled communication
- **Configuration**: Schema definitions and loading
- **Session Management**: Conversation history persistence
- **Memory System**: Long-term memory and searchable history log
- **Error Types**: Unified error handling
- **Utilities**: Common helper functions

### agent-diva-agent

The brain of the system. Provides:

- **Agent Loop**: Core processing engine
- **Context Builder**: Assembles prompts for LLM
- **Skill Loader**: Loads and manages skills
- **Subagent Manager**: Handles background tasks

### agent-diva-providers

LLM provider integrations. Provides:

- **Provider Trait**: Abstraction for LLM providers
- **LiteLLM Client**: HTTP client for LiteLLM-compatible APIs
- **Provider Registry**: Registration and lookup of providers
- **Transcription Service**: Voice-to-text via Groq Whisper

### agent-diva-channels

Chat platform integrations. Provides:

- **Channel Handler Trait**: Common interface for all channels
- **Channel Manager**: Lifecycle management of channels
- **Platform Handlers**: Telegram, Discord, Slack, etc.

### agent-diva-tools

Built-in tool implementations. Provides:

- **Tool Trait**: Interface for all tools
- **Tool Registry**: Registration and lookup
- **Tool Implementations**: Filesystem, shell, web, etc.

### agent-diva-cli

Command-line interface. Provides:

- **Commands**: onboard, gateway, agent, status, channels, cron
- **Interactive Mode**: REPL for direct interaction
- **Output Formatting**: Rich terminal output

### agent-diva-migration

Migration tool from Python version. Provides:

- **Config Migration**: Convert Python config to Rust format
- **Session Migration**: Convert Python sessions to Rust format
- **Dry-run Mode**: Preview changes without applying

## Data Flow

### Incoming Message Flow

```
Channel Handler
      │
      ▼
Message Bus (inbound queue)
      │
      ▼
Agent Loop
      │
      ├─► Context Builder (assemble prompt)
      │
      ├─► LLM Provider (get response)
      │
      ├─► Tool Execution (if needed)
      │
      ▼
Message Bus (outbound queue)
      │
      ▼
Channel Handler (send response)
```

### Session Persistence Flow

```
Agent Loop
      │
      ▼
Session Manager
      │
      ├─► In-memory cache (fast access)
      │
      └─► JSONL file (persistent storage)
```

### Memory Access Flow

```
Context Builder
      │
      ▼
Memory Manager
      │
      ├─► MEMORY.md (long-term memory)
      │
      └─► HISTORY.md (append-only memory history)
```

## Async Architecture

Agent Diva uses Tokio as its async runtime with the following patterns:

- **Multi-threaded scheduler**: `rt-multi-thread` for I/O-bound operations
- **Channels**: `tokio::sync::mpsc` for message passing
- **Task spawning**: `tokio::spawn` for concurrent operations
- **Graceful shutdown**: Signal handling for clean termination

## Error Handling

We use a layered error handling approach:

- **thiserror**: For library error types (agent-diva-core, etc.)
- **anyhow**: For application error handling (agent-diva-cli)
- **Structured errors**: Specific error types for different failure modes

## Configuration

Configuration is loaded from multiple sources (in order of precedence):

1. Environment variables (`AGENT_DIVA__*`)
2. Configuration file (`~/.agent-diva/config.json`)
3. Default values

## Security Considerations

- **Workspace restriction**: Tools can be restricted to workspace directory
- **Path validation**: All file operations validate paths
- **Allowlists**: Channels support user allowlists
- **No secrets in logs**: API keys are redacted from logs

## Performance Considerations

- **Zero-copy where possible**: Using `Cow<str>` for string handling
- **Connection pooling**: HTTP clients reuse connections
- **Caching**: Tool schemas and skills are cached
- **Lazy loading**: Sessions loaded on demand

## Testing Strategy

- **Unit tests**: In-module tests for individual functions
- **Integration tests**: Cross-crate functionality
- **Mocking**: External services mocked for tests
- **CI/CD**: Automated testing on multiple platforms
