# C1a Typed Stable Prefix Summary

## 完成内容

- 将 C1-0 `PromptSection` / `ContextSection` 接入生产上下文装配。
- 把 stable 内容拆为 Mask/Identity、Frozen Core、Rules/Skills、Memory Policy/Index，
  按固定顺序合并为唯一首条 system message。
- 将 Current Time/session、Working Memory、Recall、Plan、Ask、Scheduled 移到 history
  后、current user 前的动态 envelope，删除相关 `insert(1)`。
- 增加 provider 动态上下文 transport capability；未知 provider 安全回退到
  `UserContextEnvelope`，Native transport 在通用消息路径 fail closed。
- reactive compaction 重建时复用同一 turn 的动态 section 快照，并同步新的
  `turn_messages_start`。

## 影响范围

- `agent-diva-agent` 的 system prompt、turn context、溢出压缩重建与相关测试。
- `agent-diva-providers` 的 `LLMProvider` 能力契约及 Anthropic/OpenAI-compatible/Ollama
  adapter characterization。
- `ContextBuilder::build_messages*` 现在显式生成一条 post-prefix volatile user envelope。

## 明确未改

- 工具定义排序与 schema 锁（C1b）。
- SessionStable section cache（C1c）。
- hash/cache usage 观测与 `apply_cache_control`（C1d）。
