# 验收

## 本片完成

- `memory_list` / `memory_add` / `memory_search` 返回的 `id` 是完整原始值。
- 模型可以用该 id 做 `memory_update` / `memory_remove`，不再因为 `[REDACTED:Phone]` 找不到记录。
- 内容里真正的手机号、卡号仍会被脱敏。

## 操作步骤

1. 重启当前 gateway / agent 进程（加载新的 `agent-diva-core` 脱敏逻辑）。
2. 让模型 `memory_list` 已有记录，确认 `id` 形如 `memory-17…-xxxxxxxxxxxx`，中间没有
   `[REDACTED:…]`。
3. 新增一条内容不同的记录，确认新 id 同样完整。
4. 用返回的原始 id 做一次 update，再做一次 remove，两次都应命中记录。

## 仍挂人工

`MEMORY-CRUD-ID-DESKTOP-SMOKE`：真实模型对话里走完增删改。
