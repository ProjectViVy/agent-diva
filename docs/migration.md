# Migration Guide

This guide helps you migrate from the Python version of agent-diva to the Rust version.

## Overview

The Rust version of agent-diva maintains compatibility with the Python version's:
- Configuration format
- Session storage format
- Workspace structure

However, there are some differences to be aware of.

## Using the Migration Tool

The easiest way to migrate is using the built-in migration tool:

```bash
# Install the migration tool
cargo install --path agent-diva-migration

# Run migration (dry-run first)
agent-diva-migrate --dry-run

# Apply migration
agent-diva-migrate
```

## Manual Migration

### Configuration Migration

The configuration format is mostly compatible. Key differences:

#### Python config.json
```json
{
  "agents": {
    "defaults": {
      "workspace": "~/.agent-diva/workspace",
      "model": "anthropic/claude-opus-4-5"
    }
  },
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "YOUR_TOKEN"
    }
  }
}
```

#### Rust config.json
```json
{
  "agents": {
    "defaults": {
      "workspace": "~/.agent-diva/workspace",
      "model": "anthropic/claude-opus-4-5",
      "max_tokens": 8192,
      "temperature": 0.7,
      "max_tool_iterations": 20
    }
  },
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "YOUR_TOKEN",
      "allow_from": [],
      "proxy": null
    }
  }
}
```

The Rust version adds some new fields but accepts the old format with defaults.

### Session Migration

Session files are stored in the same location (`~/.agent-diva/sessions/`) and use the same JSONL format. No migration is needed for sessions.

### Workspace Migration

The workspace structure remains the same:

```
~/.agent-diva/workspace/
├── AGENTS.md
├── SOUL.md
├── USER.md
├── TOOLS.md
├── HEARTBEAT.md
└── memory/
    ├── MEMORY.md
    └── 2024-01-15.md
```

### Environment Variables

Environment variable names have changed:

| Python | Rust |
|--------|------|
| `AGENT_DIVA_TELEGRAM_TOKEN` | `AGENT_DIVA__CHANNELS__TELEGRAM__TOKEN` |
| `AGENT_DIVA_OPENAI_API_KEY` | `AGENT_DIVA__PROVIDERS__OPENAI__API_KEY` |
| `AGENT_DIVA_WORKSPACE` | `AGENT_DIVA__AGENTS__DEFAULTS__WORKSPACE` |

The Rust version uses double underscores (`__`) for nested configuration.

## Breaking Changes

### CLI Commands

Some CLI commands have changed:

| Python | Rust | Notes |
|--------|------|-------|
| `agent-diva chat` | `agent-diva agent` | Renamed for clarity |
| `agent-diva serve` | `agent-diva gateway` | Renamed for clarity |
| `Agent Diva config` | (removed) | Edit config.json directly |

### API Changes

If you were using agent-diva as a library (Python), the API has completely changed. You'll need to rewrite integration code.

### Plugin System

The Python version supported dynamic plugin loading. The Rust version uses a different approach:

- **Static linking**: Tools and channels are compiled in
- **Skills**: Still loaded dynamically from Markdown files
- **Future**: WASM-based plugin system planned

## Feature Parity

### Channels

| Channel | Python | Rust |
|---------|--------|------|
| Telegram | ✅ | ✅ |
| Discord | ✅ | ✅ |
| Slack | ✅ | ✅ |
| WhatsApp | ✅ | ✅ |
| Feishu | ✅ | ✅ |
| DingTalk | ✅ | ✅ |
| Email | ✅ | ✅ |
| QQ | ✅ | ✅ |

### Providers

| Provider | Python | Rust |
|----------|--------|------|
| OpenRouter | ✅ | ✅ |
| Anthropic | ✅ | ✅ |
| OpenAI | ✅ | ✅ |
| DeepSeek | ✅ | ✅ |
| Groq | ✅ | ✅ |
| Gemini | ✅ | ✅ |
| Zhipu | ✅ | ✅ |
| DashScope | ✅ | ✅ |
| Moonshot | ✅ | ✅ |
| AiHubMix | ✅ | ✅ |
| vLLM | ✅ | ✅ |

### Tools

| Tool | Python | Rust |
|------|--------|------|
| read_file | ✅ | ✅ |
| write_file | ✅ | ✅ |
| edit_file | ✅ | ✅ |
| list_dir | ✅ | ✅ |
| shell | ✅ | ✅ |
| web_search | ✅ | ✅ |
| web_fetch | ✅ | ✅ |
| message | ✅ | ✅ |
| spawn | ✅ | ✅ |
| cron | ✅ | ✅ |

## Post-Migration Checklist

After migrating:

- [ ] Verify configuration loads correctly
- [ ] Test each enabled channel
- [ ] Verify sessions are accessible
- [ ] Test a few conversations
- [ ] Check that skills load correctly
- [ ] Verify tools work as expected

## Rollback

If you need to rollback to the Python version:

1. Stop the Rust gateway
2. Reinstall Python version: `pip install agent-diva-ai`
3. Your configuration and sessions are compatible

## Getting Help

If you encounter issues during migration:

1. Check the [troubleshooting guide](troubleshooting.md)
2. Open an issue on GitHub
3. Include the output of `agent-diva-migrate --dry-run`

## Migration Timeline

The Python version will be maintained for:
- **3 months**: Full support
- **6 months**: Critical bug fixes only
- **After 6 months**: Community support only

We recommend migrating as soon as possible to benefit from performance improvements and new features.
