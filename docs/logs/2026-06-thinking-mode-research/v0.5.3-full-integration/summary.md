# Phase 2+3+4: Full Thinking Mode Integration

## Summary

完成了 thinking mode 的完整集成：配置层 → 运行时控制 → CLI 命令 → GUI 渲染。

## Phase 2: ReasoningConfig + ThinkingMode (agent-diva-core)

- `agent-diva-core/src/reasoning.rs` (NEW): ThinkingMode enum (Auto/On/Off), ReasoningConfig, ThinkingTokenLimits
- `agent-diva-core/src/lib.rs`: +pub mod reasoning
- `agent-diva-core/src/config/schema.rs`: +thinking_mode field on AgentDefaults

## Phase 3: Runtime Control + CLI Command

- `agent-diva-agent/src/runtime_control.rs`: +RuntimeControlCommand::SetThinking
- `agent-diva-agent/src/agent_loop.rs`: +thinking_mode field + init in 3 constructors
- `agent-diva-agent/src/agent_loop/loop_runtime_control.rs`: +SetThinking handler
- `agent-diva-agent/src/agent_loop/loop_turn.rs`: +ThinkingMode import, Off check clears reasoning_content
- `agent-diva-cli/src/chat_commands.rs`: +ThinkingMode import, /thinking auto|on|off command

## Phase 4: GUI ThinkingBlock (agent-diva-gui)

- `agent-diva-gui/src/components/chat/ThinkingBlock.vue` (NEW): collapsible thinking block with 🧠 icon, duration, expand/collapse
- `agent-diva-gui/src/components/ChatView.vue`: replaced old inline reasoning UI with ThinkingBlock component
- `agent-diva-gui/src/locales/en.ts`: +chat.thinkingProcess
- `agent-diva-gui/src/locales/zh.ts`: +chat.thinkingProcess

## Changed Files

12 files, +~130 / -~42 lines across backend and frontend.

## Validation

- cargo check -p agent-diva-core,agent-diva-agent,agent-diva-cli: clean ✅
- cargo fmt: applied ✅
- cargo test -p agent-diva-agent: 72/73 passed (1 pre-existing failure unrelated) ✅
- cargo test -p agent-diva-providers: all passed ✅
