# Release: v0.4.11-chat-commands-collapsible-match

## 发布方式

本次为内部代码质量修复，无需额外发布流程变更，沿用现有常规发布流程即可。

## 发布前提

- `just check` 通过

## 回滚方式

如需回滚，仅需回退 `agent-diva-cli/src/chat_commands.rs` 中本次结构性重构提交，不涉及数据迁移或配置迁移。
