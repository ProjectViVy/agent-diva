# Session 存储实现对比研究

> 生成时间: 2026-06-01
> 对比项目: claude-code (Anthropic 官方 CLI)、openfang (Rust agent 框架)、OpenHarness (Python runtime)、agent-diva (本项目)

---

## 1. claude-code (优先级最高)

### 1.1 整体架构

```
~/.claude/projects/<sanitized-cwd>/<sessionId>.jsonl
```

- **存储位置**: `~/.claude/projects/` 下，按 sanitized 的 CWD 路径分目录
- **文件格式**: JSONL (每行一个 JSON 对象)，权限 0o600
- **Session ID**: UUID v4，通过 `randomUUID()` 生成
- **Session 目录**: 由 `sessionProjectDir` 和 `originalCwd` 共同决定，支持 git worktree 跨目录 session

### 1.2 写入机制 (核心亮点)

使用 `Project` 单例类管理，关键设计:

1. **延迟物化 (Lazy Materialize)**: session 文件不会立即创建，`pendingEntries` 缓冲区暂存 hook message/attachment 等元数据。只有第一条 `user` 或 `assistant` 消息才触发 `materializeSessionFile()`。

2. **Batch 写入队列**: 每条消息通过 `enqueueWrite()` 放入 per-file 写队列:
   - 队列上限 1000 条 (超限丢弃最老条目并 resolve)
   - `FLUSH_INTERVAL_MS = 100` (普通) / `10` (CCR v2 远程模式)
   - 每 `FLUSH_INTERVAL_MS` 调度一次 `drainWriteQueue()`
   - 100MB 最大 chunk，超限分片写入

3. **UUID 去重**: 维护 `messageSet: Set<UUID>` 内存集合，避免重复写入（fork/resume/compaction 场景）

4. **双轨道持久化**: 
   - 本地: JSONL 文件 append
   - 远程: Session Ingress API (v1，HTTP POST) 或 CCR v2 Internal Event (WebSocket)
   - 远程失败 → `gracefulShutdownSync(1)` 硬退出

5. **Tombstone 删除**: `removeMessageByUuid()` 两步走:
   - 快路径: 读文件尾 64KB，找到 UUID 匹配行，`ftruncate` + 重写尾部
   - 慢路径: 全文件读写过滤 (>50MB 跳过)

6. **Metadata 重追加**: `reAppendSessionMetadata()` 在 compaction 后和 session 退出时，将 title/tag/agent-name/mode/pr-link 等 metadata 重写到文件末尾，确保 64KB 尾窗口内可见

### 1.3 读取机制

1. **Lite 读取** (`readSessionLite` / `readHeadAndTail`):
   - 只读文件头尾各 64KB (`LITE_READ_BUF_SIZE = 65536`)
   - 从尾部提取 `customTitle`, `tag`, `last-prompt`, `aiTitle`
   - 从头部提取 `firstPrompt` (跳过非用户消息/IDE元数据/内置命令)
   - 用于 `--resume` 列表、session picker、`claude ps`

2. **完整加载** (`loadTranscriptFile`):
   - 从最后一个 compact boundary 之后开始读取 (通过 `readTranscriptForLoad`)
   - 扫描尾部 64KB 找 `"compact_boundary"` 标记
   - >5MB 文件跳过 precompact (大数据文件一定有 compact)
   - 1MB 块大小向前读取 (`TRANSCRIPT_READ_CHUNK_SIZE`)
   - 最多支持 50MB 文件读取 (`MAX_TRANSCRIPT_READ_BYTES`)

3. **链式恢复** (`buildConversationChain`): 通过 `parentUuid` 指针重建对话链，支持 compact boundary、snip、fork 等场景

### 1.4 失败恢复

- **进程崩溃**: 最后 flush 之前的消息可能丢失 (100ms 窗口)
- **Cleanup handler**: `registerCleanup()` 在退出时 flush + reAppendSessionMetadata
- **远程同步**: 先 hydrate 远程日志到本地，再启用远程持久化写入
- **split-brain 防护**: `sessionId` 和 `sessionProjectDir` 原子切换 (`switchSession`)

### 1.5 前端缓存

- Project 单例缓存当前 session 的 title/tag/agentName/mode 等 metadata
- `messageSet: Set<UUID>` 内存去重
- `existingSessionFiles: Map` 缓存 session path (上限 200)
- `planSlugCache: Map` session slug 缓存
- 无独立前端 KV 缓存/TTL 层

---

## 2. openfang

### 2.1 整体架构

```
SQLite 数据库 (通过 MemorySubstrate → SessionStore)
├── sessions 表: id, agent_id, messages (MessagePack BLOB), context_window_tokens, label, created_at, updated_at
├── canonical_sessions 表: agent_id, messages (BLOB), compaction_cursor, compacted_summary, updated_at
└── JSONL mirror: <sessions_dir>/<sessionId>.jsonl (best-effort 导出)
```

- **存储引擎**: rusqlite (SQLite)，WAL mode + busy_timeout=5000
- **序列化**: rmp_serde (MessagePack) for messages BLOB
- **连接管理**: `Arc<Mutex<Connection>>` 共享连接

### 2.2 写入机制

1. **立即写入**: `save_session()` 直接执行 `INSERT ... ON CONFLICT DO UPDATE`
2. **异步写入**: `save_session_async()` 通过 `tokio::task::spawn_blocking` 避免阻塞 async runtime
3. **Session 创建**: `create_session()` 先生成 SessionId，再 `save_session()` 持久化空 session
4. **JSONL Mirror**: `write_jsonl_mirror()` 作为 best-effort 人类可读导出，不影响主存储

### 2.3 读取机制

1. **按 Session ID**: `get_session(session_id)` → SQL SELECT + rmp_serde 反序列化
2. **按 Label**: `find_session_by_label(agent_id, label)` 
3. **列出所有 sessions**: `list_sessions()` 返回 metadata (session_id, agent_id, message_count, created_at, label)
4. **列出 agent sessions**: `list_agent_sessions(agent_id)`

### 2.4 Canonical Sessions (跨频道持久内存)

这是 openfang 最独特的设计:
- 每个 agent 有一个 `canonical_session`，跨频道共享 (Telegram/Discord/CLI)
- `append_canonical()` 追加新消息，超过阈值 (默认 100) 触发 compaction
- Compaction: 文本摘要 (truncate 200 chars each, 总摘要 ≤4000 chars) + 保留最近 50 条
- `canonical_context()`: 返回 `(compacted_summary, recent_messages)`
- `store_llm_summary()`: LLM 生成的智能摘要替代文本截断

### 2.5 失败恢复

- **WAL mode**: 崩溃后 SQLite 自动恢复
- **Mutex 锁**: 防止并发写冲突
- **JSONL Mirror 独立**: 不影响主存储

### 2.6 前端缓存

- 无独立前端缓存层
- SQLite 本身提供页面缓存
- `Arc<Mutex<Connection>>` 共享连接，减少重复打开

---

## 3. OpenHarness

### 3.1 整体架构

```
.ohmo/sessions/
├── latest.json          # 始终指向最新 session (全量快照)
├── latest-<sha1>.json   # 按 session_key 索引
├── session-<id>.json    # 按 session_id 归档
└── transcript.md        # Markdown 导出
```

- **格式**: 单个 JSON 文件，包含完整快照
- **写入方式**: `atomic_write_text()` (原子写入)
- **Session ID**: `uuid4().hex[:12]` 或用户指定

### 3.2 写入机制

1. **全量快照**: 每次 `save_session_snapshot()` 写入完整的 messages + system_prompt + usage + tool_metadata
2. **原子写入**: 使用 `atomic_write_text()` 确保写入不损坏
3. **双写**: 同时写 `latest.json` 和 `session-{sid}.json`
4. **按 key 索引**: 如果提供了 `session_key`，额外写 `latest-{sha1}.json`

### 3.3 读取机制

1. **load_latest()**: 读取 `latest.json`
2. **load_by_id(session_id)**: 读取 `session-{session_id}.json`，fallback 到 `latest.json`
3. **list_snapshots()**: 扫描 `session-*.json`，按 mtime 排序，limit 20
4. **export_session_markdown()**: 导出为 Markdown 文件

### 3.4 失败恢复

- **原子写入**: 写入过程不会损坏文件
- **全量快照**: 每个 session 文件独立，不会连锁损坏
- **无增量写入**: 消息丢失=上次快照后的所有消息

### 3.5 OhmoSessionBackend

实现 `SessionBackend` 接口，作为 runtime 的 session 持久化后端:
- 方法: `save_snapshot`, `load_latest`, `list_snapshots`, `load_by_id`, `export_markdown`
- 在 cli.py 中: `--resume` 和 `--continue` 通过 `backend.load_by_id()` 恢复

---

## 4. agent-diva (当前实现)

### 4.1 整体架构

```
<workspace>/memory/
├── MEMORY.md     # 长期记忆
├── HISTORY.md    # 历史记录
└── YYYY-MM-DD.md # 每日笔记
```

- **存储引擎**: 文件系统 (Markdown 文件)
- **MemoryManager**: 管理 MEMORY.md 和 HISTORY.md 的读写
- **MemoryProvider trait**: 抽象的 provider 接口

### 4.2 当前缺失

- **无会话级消息持久化**: 没有 session 存储 (没有 JSONL、SQLite、或其他会话格式)
- **消息只存在于内存**: 进程崩溃 = 全部丢失
- **无会话恢复机制**: 没有 `--resume` 功能
- **无增量写入/批量写入**: 没有持久化消息的管道

### 4.3 MemoryProvider 接口

定义了 4 个方法:
- `system_prompt_block()`: 启动时注入长期记忆
- `prefetch()`: 意图感知的 recall
- `sync_turn()`: 完成后持久化记忆更新
- `on_session_end()`: 会话结束处理

---

## 5. 对比表格

| 维度 | claude-code | openfang | OpenHarness | agent-diva |
|------|-----------|----------|-------------|-----------|
| **权威数据源** | JSONL 文件 (本地) | SQLite 数据库 | JSON 文件 (本地) | 内存 (无持久化) |
| **存储格式** | JSONL (每行一条消息) | MessagePack BLOB in SQLite | 单个 JSON 全量快照 | N/A |
| **写入时机** | Batch (100ms/10ms) | 立即写入 | 整会话结束时写入 | N/A |
| **增量/全量** | 增量 append | 增量 update BLOB | 全量覆盖快照 | N/A |
| **消息写入粒度** | 每条消息 queue → 批量 flush | 每条消息更新 BLOB | 整个 messages 数组 | N/A |
| **原子性** | posix append + ftruncate | SQLite WAL | atomic_write_text() | N/A |
| **远程同步** | Session Ingress v1 / CCR v2 | 无内置 | 无内置 | N/A |
| **前端缓存** | messageSet(UUID去重) + metadata cache | 无 | 无 | MemoryManager 缓存 |
| **失败恢复** | 100ms 窗口内丢失 | WAL 自动恢复 | 全量快照之间丢失 | 全部丢失 |
| **Compaction** | compact_boundary + summary + preservedSegment | 文本摘要 + cursor + 保留最近 50 条 | 无 | 无 |
| **跨频道持久内存** | 无 (per-channel session) | canonical_sessions 表 | 无 | 无 |
| **Session 列表** | 64KB 头尾扫描 (lite) | SQL SELECT metadata | glob 文件 + mtime 排序 | N/A |
| **Session 恢复** | parentUuid 链 + compact boundary skip | get_session() 反序列化 | load_by_id() | N/A |
| **Subagent 隔离** | 独立 subagents/ 目录 JSONL | 无独立机制 | 无 | N/A |

---

## 6. agent-diva 可借鉴的最佳实践

### 6.1 强烈建议采用: claude-code 的 JSONL + 批量写入模式

理由: agent-diva 是 Rust 项目（与 openfang 同类），但 claude-code 的 JSONL 模式有几个 Rust 友好的优势:

1. **JSONL append-only 简单可靠**: 不需要 SQLite 依赖，不需要 MessagePack 序列化
   - Rust 实现: `serde_json::to_writer(&mut file, &entry)` + `writeln!(file)`
   - 支持 `tokio::fs::OpenOptions::append(true)` 异步写入

2. **Batch flush 减少 I/O**: 
   - 借鉴 `Project` 类的队列设计: `VecDeque<(Entry, oneshot::Sender<()>)>` 
   - `FLUSH_INTERVAL_MS` = 100ms (可配置)，避免每条消息都 fsync
   - 队列上限防止内存无限增长

3. **UUID 去重**: `HashSet<Uuid>` 防止 compaction/resume/fork 重复写入

4. **Lite 读取**: 64KB 头尾扫描用于 session list，避免全量加载
   - 尾部提取 metadata (title, tag, mode)
   - 头部提取 firstPrompt

5. **延迟物化**: 不立即创建 session 文件，第一条 user/assistant 消息才创建

### 6.2 建议采用: openfang 的 canonical_session + compaction

理由: agent-diva 已有 `MemoryProvider` trait 和 `HISTORY.md`，非常适合集成 canonical session:

1. **跨频道持久内存**: 如果 agent-diva 未来支持多渠道 (CLI + 钉钉 + 其他)，canonical session 是必备功能
   - 可复用 agent-diva 已有的 `MemoryProvider::sync_turn()` 接口
   - 将 compaction 后的 summary 写入系统提示

2. **LLM 驱动的 compaction**: 比纯文本截断更智能
   - `store_llm_summary()` 可集成到 agent 的 post-turn hook
   - 默认阈值 100 条消息, 保留最近 50 条

### 6.3 建议考虑: OpenHarness 的 session key 索引

- `session_key` → `latest-{sha1}.json` 的映射
- 如果有多个客户端 (CLI + Web UI + 钉钉) 需要各自的 "latest session" 指针

### 6.4 具体实现建议

```rust
// 建议的 agent-diva session 存储结构
// 路径: <workspace>/sessions/<sessionId>.jsonl

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot, Mutex};
use uuid::Uuid;

struct SessionStore {
    workspace_root: PathBuf,
    session_id: Uuid,
    session_file: Option<PathBuf>,
    
    // Batch write system (借鉴 claude-code)
    write_queue: VecDeque<WriteEntry>,
    flush_interval_ms: u64,
    max_queue_size: usize,       // 1000
    max_chunk_bytes: usize,      // 100MB
    
    // Dedup (借鉴 claude-code)
    written_uuids: HashSet<Uuid>,
    
    // Metadata cache (借鉴 claude-code)
    title: Option<String>,
    tag: Option<String>,
}

// Message format (JSONL line)
#[derive(Serialize)]
struct SessionEntry {
    #[serde(rename = "type")]
    entry_type: String,  // "user" | "assistant" | "system" | "custom-title" | "tag" | ...
    uuid: Uuid,
    parent_uuid: Option<Uuid>,
    session_id: Uuid,
    timestamp: String,
    version: String,
    // ... message-specific fields
}
```

### 6.5 不推荐的做法

- **全量快照 (OpenHarness 模式)**: 对长对话不友好 (每次写入整个 messages 数组)
- **纯内存 (当前 agent-diva)**: 无崩溃恢复能力
- **同步 fsync 每条消息**: 性能差

---

## 7. 总结

| 特性 | agent-diva 当前状态 | 推荐方案 | 参考来源 |
|------|-------------------|---------|---------|
| 会话消息持久化 | 无 | JSONL append-only | claude-code |
| 写入策略 | N/A | Batch flush (100ms) | claude-code |
| 会话恢复 | 无 | parentUuid 链 + compact boundary | claude-code |
| 跨频道内存 | N/A | canonical_session (SQLite) | openfang |
| Context compaction | 无 | LLM summary + 保留窗口 | openfang + claude-code |
| Session 列表 | N/A | 64KB 头尾 lite 扫描 | claude-code |
| 远程同步 | 无 | 后续考虑 (不紧急) | claude-code |
| 长期记忆 | MEMORY.md / HISTORY.md | 保留现有 + 集成 canonical session | agent-diva + openfang |
