# Session History Storage Research — 最终调研报告

**日期**: 2026-06-01
**调研方式**: 3 路并行子代理深度审计
**范围**: agent-diva 后端 (store/manager/loop_turn/handlers) + GUI 前端 (App.vue/localStorage) + 参考项目对比 (claude-code/openfang/OpenHarness)

---

## Executive Summary

**结论：agent-diva 短期会话存储存在严重一致性问题，确认"短期记忆存不住内容"是真 bug。**

共发现 **27 个 bug**（后端 15 个、GUI 前端 12 个），其中：
- **P0（必须立即修复）**: 9 个 — 包括用户消息丢失、崩溃时文件损坏、缓存优先读旧数据、optimistic UI 永不清理等
- **P1（建议尽快修复）**: 10 个
- **P2（可延期）**: 8 个

**根本原因有三层叠加：**
1. **后端写入时机太晚** — 用户消息和 assistant 响应都在 turn 完全结束后才写入 session 文件，中途任何失败都丢失
2. **GUI 缓存优先反模式** — loadSession 先读 localStorage（30 分钟 TTL），后端更新完全不可见
3. **非原子写入 + 无错误恢复** — JSONL 全量覆盖写，save 失败只打日志不作补偿

---

## 1. 当前 agent-diva Session 数据流

```
┌─────────────────────────────────────────────────────────────────────┐
│                          WRITE PATH                                  │
│                                                                      │
│  User Input (HTTP/GUI)                                               │
│       │                                                              │
│       ▼                                                              │
│  handlers::chat_handler ──► ManagerCommand::Chat                     │
│       │                                                              │
│       ▼                                                              │
│  AgentLoop::process_inbound_message_inner                             │
│       │                                                              │
│       ├─[1] sessions.get_or_create("gui:chat-xxx")  ← cache or disk │
│       ├─[2] history = session.get_history(50)       ← read context   │
│       ├─[3] context.build_messages(history, msg)    ← LLM context    │
│       ├─[4] LLM loop (stream → tool_calls → iterate)                 │
│       │      ├─ cancellation → return Ok(None)  ← USER MSG LOST!    │
│       │      ├─ streaming error → return Ok(None)                    │
│       │      └─ tool error → return Ok(None)                         │
│       ├─[5] save_turn(session, messages)  ← FINALLY write user msg  │
│       ├─[6] consolidation::consolidate(session) ← advance index     │
│       └─[7] sessions.save(session)  ← FULL OVERWRITE JSONL          │
│                                                                      │
│  BUG: Steps [5][6][7] only run on success. Steps [1]-[4] failures   │
│       → user messages NEVER written to disk.                        │
│  BUG: Step [6] advances last_consolidated BEFORE step [7] saves.    │
│       Crash between [6] and [7] → split-brain: memory provider has  │
│       consolidation but disk doesn't → double consolidation.        │
│  BUG: Step [7] uses std::fs::write (non-atomic). Crash mid-write    │
│       → file is corrupted/empty.                                    │
│                                                                      │
├─────────────────────────────────────────────────────────────────────┤
│                          READ PATH (Manager API)                     │
│                                                                      │
│  GUI: invoke("get_session_history", { chatId })                      │
│       │                                                              │
│       ▼                                                              │
│  handlers::get_session_history_handler                               │
│       ├─ id 不含 ':' → 加 "gui:" 前缀 (BUG: 非 GUI channel 误判)    │
│       └─ ManagerCommand::GetSessionHistory                           │
│             │                                                        │
│             ▼                                                        │
│  AgentLoop::handle_runtime_control_command                           │
│       └─ sessions.get_or_load(key) → clone → oneshot reply          │
│                                                                      │
├─────────────────────────────────────────────────────────────────────┤
│                          READ PATH (GUI Cache)                       │
│                                                                      │
│  loadSession(sessionKey)                                             │
│       │                                                              │
│       ├─[1] readSessionFromCache(sessionKey) ← CACHE FIRST!          │
│       │      └─ TTL = 30min. If HIT → return stale data (BUG!)      │
│       │                                                              │
│       └─[2] MISS → invoke("get_session_history") ← backend           │
│              └─ writeSessionToCache(data) ← ONLY cache write site    │
│                                                                      │
│  BUG: writeSessionToCache NEVER called from sendMessage,             │
│       stopMessage, or streaming event handlers. Cache permanently    │
│       stale after first load.                                        │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. 后端 Session Store 关键发现

### 2.1 核心代码位置

| 文件 | 职责 |
|------|------|
| `agent-diva-core/src/session/store.rs` | Session 数据结构、ChatMessage 结构、get_history() |
| `agent-diva-core/src/session/manager.rs` | SessionManager：HashMap 缓存 + JSONL 文件读写 |
| `agent-diva-agent/src/agent_loop/loop_turn.rs` | save_turn() 函数、agent 主循环 |
| `agent-diva-agent/src/consolidation.rs` | memory consolidation |
| `agent-diva-manager/src/handlers.rs` | HTTP API 端点 |

### 2.2 15 个后端 Bug 一览

| # | Bug | 严重度 | 数据丢失？ |
|---|-----|--------|-----------|
| 1 | 用户消息在 turn 完成前不写入 session | **P0** | YES |
| 2 | Consolidation 在 save 前执行，崩溃导致 split-brain | **P0** | YES |
| 3 | JSONL 全量覆盖写非原子，崩溃时文件损坏/清空 | **P0** | YES |
| 4 | save() 失败被静默忽略，无重试 | **P0** | YES |
| 5 | load() I/O error → None → 创建空 session 覆盖旧数据 | P1 | YES |
| 6 | JSONL 解析静默丢弃不可解析行 | P1 | YES |
| 7 | list_sessions() key 编码不可逆（`_` ↔ `:` 混淆） | P1 | 错误 key |
| 8 | Prefetch 注入打破 save_turn 偏移计算 | P1 | YES |
| 9 | Cron 消息存为 "system" 角色，被 get_history() 排除 | P1 | 上下文丢失 |
| 10 | save 失败后内存缓存与磁盘不一致 | P1 | YES (重启后) |
| 11 | get_history() 可能返回孤立的 tool/assistant 消息 | P2 | 上下文降级 |
| 12 | Tool 结果被截断到 500 字符 | P2 | 部分丢失 |
| 13 | delete() 先删缓存再删文件，文件删除失败时状态混乱 | P2 | 瞬态 |
| 14 | get_session_history_handler 对非 GUI channel 也加 "gui:" 前缀 | P2 | 查找失败 |
| 15 | save_turn 最终消息检测边界条件 | P2 | 罕见 |

### 2.3 数据丢失链（最严重场景）

```
用户发送 "帮我写代码"
  → get_or_create → 加载旧历史 ✓
  → LLM 开始流式响应
  → 第 3 次 tool call 时被用户取消
  → return Ok(None)  ← 用户消息从未写入 session
  → 用户的消息永久丢失
  → 用户刷新 GUI
  → loadSession 读缓存（有旧数据） → 显示旧的会话
  → 用户看到的消息列表里没有 "帮我写代码"
```

---

## 3. GUI 前端缓存关键发现

### 3.1 核心代码位置

| 文件 | 职责 |
|------|------|
| `agent-diva-gui/src/App.vue` | 主组件：loadSession, sendMessage, 缓存读写 |
| `agent-diva-gui/src/utils/localStorageAgentDiva.ts` | localStorage 键名常量 |

### 3.2 12 个 GUI Bug 一览

| # | Bug | 严重度 |
|---|-----|--------|
| 1 | loadSession 缓存优先，无失效机制 → 30 分钟内后端更新不可见 | **P0** |
| 2 | sendMessage 成功后从不更新 localStorage 缓存 | **P0** |
| 3 | sendMessage 失败时用户消息不从 UI 移除 | **P0** |
| 4 | 空 streaming placeholder 无超时清理，永不消失 | **P0** |
| 5 | activeStreamRequestId 覆盖导致旧 stream 事件被丢弃 | **P0** |
| 6 | tool-start/tool-end 事件处理器创建冗余 agent placeholder | P1 |
| 7 | stopMessage 不清理 placeholder 也不写缓存 | P1 |
| 8 | deleteSession 后端失败时仍从 UI 移除 | P1 |
| 9 | 30 分钟 TTL 在多实例场景过长 | P1 |
| 10 | clearMessages 不清旧 session 缓存 | P2 |
| 11 | 无跨窗口缓存失效 | P2 |
| 12 | refreshSessions 不触发缓存重新验证 | P2 |

### 3.3 缓存失效矩阵（当前状态）

```
操作                      | 更新 messages.value | 清除 localStorage 缓存 | 从后端刷新
──────────────────────────┼────────────────────┼──────────────────────┼────────────
应用启动 → loadSession     | ✓ (从缓存/后端)     | ✗                    | 仅缓存 miss 时
发送消息成功               | ✓ (streaming)       | ✗                    | ✗
发送消息失败               | ✓ (用户消息残留)    | ✗                    | ✗
停止生成                   | ✓ (标记完成)        | ✗                    | ✗
切换会话 → loadSession     | ✓ (从缓存)          | ✗                    | ✗
删除会话                   | ✓ (移除)            | ✓                    | ✗
新建会话                   | ✓ (清空)            | ✗                    | ✗
30 分钟 TTL 到期           | ✗                   | 读时自动删             | ✗
手动刷新 (无此功能)        | N/A                 | N/A                  | N/A
```

**结论：缓存只有两个操作点 — 读（loadSession）和写（loadSession cache miss 时）。sendMessage/stopMessage/switch session 全都不更新缓存。**

---

## 4. 参考项目对比结论

| 维度 | claude-code | openfang | agent-diva |
|------|-----------|----------|-----------|
| **存储格式** | JSONL append-only | SQLite + MessagePack BLOB | JSONL 全量覆盖写 |
| **写入方式** | Batch queue (100ms flush) | 立即 UPDATE BLOB | turn 完成后全量覆盖 |
| **原子性** | posix append + ftruncate | SQLite WAL | std::fs::write (非原子) |
| **写入时机** | 每条消息 queue → 批量 flush | 每条消息更新 | turn 完全结束后一次性写 |
| **前端缓存** | messageSet (UUID 去重) | 无 | localStorage 30min TTL |
| **失败恢复** | 100ms 窗口内可能丢失 | WAL 自动恢复 | 全部丢失 |
| **缓存策略** | 后端权威 | 后端权威 | **缓存优先，后端只做 fallback** |
| **Compaction** | compact_boundary + preservedSegment | 文本摘要 + cursor | last_consolidated (有 bug) |

### 4.1 agent-diva 最需要向 claude-code 学习的

1. **JSONL append-only** 替代全量覆盖写
2. **Batch flush** (100ms 去抖) 替代每条消息 fsync
3. **写时即持久化** 替代 turn 结束后批量写
4. **UUID 去重** 防止 compaction/resume 重复
5. **64KB 头尾 lite 扫描** 用于 session 列表

---

## 5. 复现矩阵

应在调研后执行的实际测试用例：

| # | 场景 | 预期问题 | 验证方法 |
|---|------|---------|---------|
| 1 | 正常发送 → 等待完成 → 刷新 | GUI 缓存可能显示旧数据 | 比较 GUI 显示和 sessions/*.jsonl |
| 2 | 发送后立即刷新（不等待响应） | 用户消息丢失（后端未写入） | 检查 session JSONL 文件 |
| 3 | Agent turn 中途取消 | 用户消息丢失 | 检查 session JSONL 文件 |
| 4 | Agent turn 工具报错 | 用户消息丢失 | 检查 session JSONL 文件 |
| 5 | 发送消息 → 等待完成 → 切换会话 → 切回 | GUI 显示旧缓存（30min TTL） | 比较 GUI 和后端 |
| 6 | 发送消息 → 完 成 → 30 分钟内 CLI 操作同一会话 → GUI 刷新 | GUI 看不到 CLI 的修改 | 比较 GUI 和后端 |
| 7 | Reset 会话后检查 localStorage | 旧缓存可能未被清理 | 检查 localStorage |
| 8 | 删除会话后检查 localStorage 和后端文件 | 后端可能未删但 UI 已移除 | 检查文件系统 |
| 9 | 模拟 backend save() 失败 | 用户收到响应但 session 未保存 | Mock 磁盘满 |
| 10 | 模拟进程在 consolidation 和 save 之间崩溃 | Split-brain | 检查 session vs memory |

---

## 6. 严重度评估和修复优先级

### P0 — 必须立即修复（9 个）

| # | 类别 | Bug | 修复建议 |
|---|------|-----|---------|
| 1 | 后端 | 用户消息不在 turn 开始前写入 | get_or_create 后立即 add_message + save |
| 2 | 后端 | Consolidation 在 save 前执行 | 调序：先 save 再 consolidate |
| 3 | 后端 | JSONL 非原子覆盖写 | 写 .tmp 文件 → rename |
| 4 | 后端 | save() 失败无重试 | 重试 + dirty flag + shutdown save |
| 5 | GUI | 缓存优先无失效 | stale-while-revalidate: 先返回缓存，后台异步刷新后端 |
| 6 | GUI | sendMessage 后不更新缓存 | agent-response-complete 时调用 writeSessionToCache |
| 7 | GUI | 失败时用户消息残留 | catch 块中同时 pop 用户消息 |
| 8 | GUI | 空 streaming placeholder 无超时 | 30 秒超时自动清理 |
| 9 | GUI | activeStreamRequestId 覆盖 | 覆盖前 closeStreamingPlaceholder 清理旧 stream |

### P1 — 建议尽快修复（10 个）

| # | 类别 | Bug |
|---|------|-----|
| 1 | 后端 | load() I/O error → 空 session |
| 2 | 后端 | JSONL 静默丢弃不可解析行 |
| 3 | 后端 | list_sessions key 编码不可逆 |
| 4 | 后端 | Prefetch 打破 save_turn 偏移 |
| 5 | 后端 | Cron 消息被 get_history 排除 |
| 6 | 后端 | save 失败后缓存与磁盘不一致 |
| 7 | GUI | tool 事件创建冗余 placeholder |
| 8 | GUI | stopMessage 不写缓存 |
| 9 | GUI | deleteSession 后端失败时 UI 仍移除 |
| 10 | GUI | 30min TTL 过长 |

### P2 — 可延期（8 个）

略（见子报告详情）

---

## 7. 推荐修复方案

### 7.1 阶段 1：确保数据不丢失（P0 后端）

```
修复顺序：
1. loop_turn.rs: 在 get_or_create 后立即写入用户消息
2. manager.rs: 改用 write-to-tmp-then-rename 原子写入
3. loop_turn.rs: save() 前先 persist，再 consolidate
4. loop_turn.rs: save() 失败时重试机制
```

### 7.2 阶段 2：确保 GUI 读到权威数据（P0 GUI）

```
修复顺序：
1. loadSession: stale-while-revalidate 模式
2. sendMessage 完成时 writeSessionToCache
3. 失败时清理 optimistic messages
4. 空 placeholder 超时清理
5. activeStreamRequestId 正确处理多 stream
```

### 7.3 阶段 3：架构升级（中期）

借鉴 claude-code 的 JSONL append-only + batch flush 模式：
- 替换当前的全量覆盖写为增量 append
- 引入 100ms flush 去抖
- UUID 去重
- Session lite 读取（头尾 64KB 扫描）

---

## 8. 测试计划

### 8.1 单元测试

- SessionManager save/load 往返测试
- save() 失败后 cache 一致性测试
- get_history() 边界测试（空 session、全 consolidated、孤儿消息）
- session_path() key 编码往返测试
- consolidation 前后 save 顺序测试

### 8.2 集成测试

- Manager API get_session_history 返回正确数据
- Agent turn 取消后 session 文件内容验证
- Agent turn 失败后 session 文件内容验证
- Prefetch 场景下 save_turn 消息保真度

### 8.3 GUI 测试

- loadSession 缓存命中/未命中路径
- sendMessage 完成后缓存更新
- sendMessage 失败后 UI 状态
- 切换会话后缓存一致性
- 30min TTL 过期后行为

### 8.4 手动冒烟

- 发送消息 → 等待完成 → 检查 sessions/*.jsonl
- 发送 → 中途关闭进程 → 重启 → 检查 session 文件
- CLI 修改会话 → GUI 是否可见
- 删除会话 → 检查 localStorage 和文件系统

---

## 9. 回答交班提出的核心问题

| 问题 | 答案 |
|------|------|
| 是否存在"短期历史存不住内容"？ | **是。** 用户消息在 turn 开始前不写入，任何取消/失败都丢失。 |
| 是否存在"GUI 读取旧缓存覆盖后端历史"？ | **是。** loadSession 缓存优先，sendMessage 后缓存从不更新。 |
| 是否存在"失败后前端 placeholder 永久污染"？ | **是。** 空 placeholder 无超时清理，用户消息在失败时不清除。 |
| 是否存在"后端写入时机导致用户输入丢失"？ | **是。** save_turn 只在 turn 完全成功后调用。 |
| 是前端 cache 问题还是后端写入问题？ | **两者叠加。** 后端写太晚 + 前端缓存优先 = 双重数据丢失。 |

---

## 10. 开放问题

1. `agent-diva-nano` 模式下的 session 存储是否使用相同链路？
2. 是否有计划支持 CLI 和 GUI 同时操作同一会话？
3. 长期记忆 (MEMORY.md) 的 sync_turn 如果失败，是否需要回滚 session 状态？
4. GUI 的 `locallyDeletedSessionKeys` Set 在页面刷新后是否重置？

---

## 附录：子报告位置

- 后端审计: `docs/logs/2026-06-session-research/subagent-backend-audit.md`
- GUI 审计: `docs/logs/2026-06-session-research/subagent-gui-audit.md`
- 参考对比: `docs/logs/2026-06-session-research/subagent-reference-comparison.md`
