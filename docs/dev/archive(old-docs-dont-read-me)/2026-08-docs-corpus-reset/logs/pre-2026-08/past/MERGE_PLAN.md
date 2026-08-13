# main → agent-diva-pro 后端合并方案

> 调查日期：2026-06-02
> 范围：main 分支相对 agent-diva-pro 分支的后端 6 个 crate 变更（53 文件，+10446/-417 行）

---

## 一、各 Crate 变更概要

### 1.1 agent-diva-core（10 文件，+1307/-6）

| 变更类型 | 文件 | 说明 |
|---------|------|------|
| **新增模块** | `src/attachment.rs` | `FileAttachment` 结构体，统一文件附件表示（355 行） |
| **新增模块** | `src/security/mod.rs` | 安全模块入口，re-export 所有安全类型 |
| **新增模块** | `src/security/config.rs` | `SecurityConfig`、`SecurityLevel`（Permissive/Standard/Strict/Paranoid） |
| **新增模块** | `src/security/error.rs` | `SecurityError` 枚举（PathNotAllowed、RateLimitExceeded、ReadOnlyMode 等） |
| **新增模块** | `src/security/path.rs` | `PathValidator` — 6 层路径安全校验（null 字节、遍历、URL 编码、tilde、绝对路径、禁止前缀） |
| **新增模块** | `src/security/policy.rs` | `SecurityPolicy` — 组合路径校验 + 速率限制的统一策略入口 |
| **新增模块** | `src/security/rate_limit.rs` | `ActionTracker` — 基于滑动窗口的文件操作速率限制 |
| **修改** | `src/lib.rs` | 新增 `pub mod attachment`、`pub mod security`、`pub use attachment::FileAttachment` |
| **修改** | `src/config/schema.rs` | `FeishuConfig` 新增 `port: Option<u16>`；`default_port()` 从 `18790` → `3000` |
| **修改** | `Cargo.toml` | 版本 → 0.4.9；新增 `parking_lot`、`agent-diva-files` 依赖 |

**关键 API 变化：**
- `SecurityPolicy::with_config(workspace, config)` 替代了原来的 `Option<PathBuf> allowed_dir` 模式
- `SecurityLevel` 四级预设控制文件工具的访问权限
- `FileAttachment` 是跨 channel 的统一附件表示，序列化为 JSON

---

### 1.2 agent-diva-files（14 文件，+5000+ 行，**全新 crate**）

全新的内容寻址文件存储系统，核心架构：

| 文件 | 行数 | 职责 |
|------|------|------|
| `lib.rs` | 112 | 公共 API 导出：`FileManager`、`FileConfig`、`FileHandle`、`FileMetadata`、`default_data_dir_or_fallback()` |
| `config.rs` | 241 | `FileConfig` 配置（存储路径、软删除保留天数、max_inline_size 等） |
| `storage.rs` | 187 | 文件 I/O + SHA256 计算，`hash_to_path()` 映射 |
| `backend.rs` | 366 | `StorageBackend` trait + `LocalStorageBackend` 实现 |
| `handle.rs` | 252 | `FileHandle`（id + path + metadata + ref_count）、`FileMetadata` 结构体 |
| `index.rs` | 655 | SQLite 索引（`files` 表），CRUD + 引用计数原子操作 |
| `manager.rs` | 813 | `FileManager` 主接口：`store()`、`get()`、`read()`、`release()`、`delete()` |
| `hooks.rs` | 1063 | Hook 系统：`HookManager`、`HookEvent`（BeforeStore/AfterStore/BeforeRead 等）、支持异步链式调用 |
| `channel.rs` | 834 | Channel 文件管理：逻辑隔离、跨 channel 共享、批量操作 |
| `README.md` | 120 | 使用文档 |
| `docs/LEARNING.md` | 558 | 详细教程（内容寻址、引用计数、SQLite 索引原理） |
| `docs/acceptance.md` | 72 | 验收标准清单 |
| `docs/debugging.md` | 441 | 调试指南 |
| `Cargo.toml` | 30 | 依赖：`sha2`、`hex`、`sqlx`（sqlite）、`dirs` |

**核心设计：**
- SHA256 内容寻址 → 文件 ID 格式 `sha256:<hash>`
- 物理存储路径由 hash 前 2 字符分目录：`ab/c123...`
- SQLite 持久化索引（`index.db`），ACID 事务保证一致性
- 引用计数：`ref_count = 0` 时才真正删除
- 软删除：`deleted_at` + `deleted_by` 标记，支持恢复

---

### 1.3 agent-diva-providers（4 文件，+647/-2）

| 变更类型 | 文件 | 说明 |
|---------|------|------|
| **新增** | `src/ollama.rs` | `OllamaProvider` — 完整的 Ollama 本地模型 provider（493 行） |
| **新增** | `tests/ollama_streaming.rs` | SSE 流式聊天集成测试 |
| **新增** | `tests/ollama_tools.rs` | Tool calling 集成测试 |
| **修改** | `Cargo.toml` | 版本 → 0.4.9 |

**OllamaProvider 特性：**
- 实现 `LLMProvider` trait（`chat()` + `chat_stream()`）
- SSE 流式解析：逐 chunk 解析 `OllamaStreamChunk`
- Tool calling：支持 Ollama 原生 tool_calls 格式 → `ToolCallRequest` 转换
- Thinking/Reasoning 支持：`reasoning_content` 字段
- URL 标准化：自动去除 `/api` 后缀，默认 `http://localhost:11434`
- 300 秒超时（本地模型推理较慢）

---

### 1.4 agent-diva-channels（6 文件，+1462/-301）

| 变更类型 | 文件 | 说明 |
|---------|------|------|
| **重写** | `src/feishu.rs` | 从 JSON WebSocket → protobuf（pbbp2.proto）帧编码（+623/-原有） |
| **重写** | `src/qq.rs` | 完整重写连接管理：Session Resume、指数退避、Heartbeat ACK 超时（+759/-原有） |
| **新增** | `tests/qq_reconnect_integration.rs` | QQ 重连集成测试（783 行） |
| **微调** | `src/irc.rs` | SASL 认证 match 简化（guard 提前） |
| **微调** | `src/dingtalk.rs` | User-Agent 版本号更新 |
| **修改** | `Cargo.toml` | 新增 `prost`、`prost-derive`、`futures-util`；`lettre` TLS 从 `native-tls` → `rustls-tls`；非 Windows 添加 `openssl = { features = ["vendored"] }` |

**飞书 Channel 重大变更：**
- WebSocket 连接从 `GET /bot/v2/websocket` → `POST /callback/ws/endpoint`（新协议）
- 二进制 protobuf 帧编码（`PbFrame` 结构体：seq_id, log_id, service, method, headers, payload）
- `method=0` 控制帧（ping/pong），`method=1` 数据帧（事件）
- 心跳机制：可配置 `ping_interval`，300 秒超时断连
- 消息分片重组：`sum`/`seq` 字段 + `frag_cache`
- 去重改为 `HashMap<String, Instant>` + TTL 自动清理（替代 `VecDeque`）
- 新增图片消息处理：`fetch_image_marker()` 下载 → base64 编码

**QQ Channel 重大变更：**
- 新增 `SessionState`（session_id + last_sequence）持久化
- `AttemptMode`（Identify/Resume）+ `ExitReason` 枚举
- 指数退避：`INVALID_SESSION_BACKOFF_SECS = [5, 15, 30, 60]`
- 连续 5 次 invalid session → 300 秒冷却
- `running` 从 `AtomicBool` → `RwLock<bool>`
- 环境变量覆盖：`QQ_ACCESS_TOKEN_OVERRIDE`、`QQ_GATEWAY_URL_OVERRIDE`、`QQ_API_BASE_OVERRIDE`

---

### 1.5 agent-diva-agent（5 文件，+218/-23）

| 变更类型 | 文件 | 说明 |
|---------|------|------|
| **修改** | `src/agent_loop.rs` | `AgentLoop::new()` 和 `with_tools()` 改为 `async`，返回 `Result`；新增 `file_manager: Arc<FileManager>` 字段 |
| **修改** | `src/agent_loop/loop_tools.rs` | 文件工具从 `Option<PathBuf>` → `Arc<SecurityPolicy>`；新增 `ReadAttachmentTool` |
| **修改** | `src/agent_loop/loop_turn.rs` | 新增 `load_attachment_contents()` 方法：文本附件内联到消息，大文件提示用 tool |
| **修改** | `src/subagent.rs` | 子 agent 同步迁移到 `SecurityPolicy` 模式 |
| **修改** | `Cargo.toml` | 新增 `agent-diva-files`、`dirs` 依赖 |

**关键 API 变化：**
```rust
// Before (pro)
pub fn new(bus, provider, workspace, model, max_iterations) -> Self
pub fn with_tools(bus, provider, workspace, model, max_iterations, tool_config, runtime_control_rx) -> Self

// After (main)
pub async fn new(bus, provider, workspace, model, max_iterations) -> Result<Self, Box<dyn Error>>
pub async fn with_tools(bus, provider, workspace, model, max_iterations, tool_config, runtime_control_rx, file_manager) -> Result<Self, Box<dyn Error>>
```

- 文件系统工具构造：`ReadFileTool::new(security)` 替代 `ReadFileTool::new(allowed_dir)`
- 附件内联策略：≤100KB 的 text/*、application/json 等直接内联；其他文件提示 AI 使用 `read_file` tool
- `MAX_INLINE_ATTACHMENT_SIZE = 100 * 1024`

---

### 1.6 agent-diva-cli（3 文件，+47/-11）

| 变更类型 | 文件 | 说明 |
|---------|------|------|
| **修改** | `src/main.rs` | TUI/Status 命令初始化 `FileManager`；版本号 → 0.4.9 |
| **修改** | `src/chat_commands.rs` | `build_local_cli_agent()` 改为 `async`，初始化 `FileManager` |
| **修改** | `Cargo.toml` | 新增 `agent-diva-files` 依赖 |

---

### 1.7 agent-diva-manager（11 文件，+364/-30）

| 变更类型 | 文件 | 说明 |
|---------|------|------|
| **新增** | `src/file_service.rs` | `FileService` — 文件上传/下载服务层（147 行） |
| **修改** | `src/handlers.rs` | 新增 `upload_file_handler()`；`ChatRequest` 增加 `attachments` 字段 |
| **修改** | `src/lib.rs` | 导出 `file_service`、`start_embedded_gateway_runtime`、`EmbeddedGatewayRuntime`、`build_router`、`run_server_with_listener` |
| **修改** | `src/manager.rs` | `Manager` 新增 `file_manager` 字段；处理 `UploadFile` 命令 |
| **修改** | `src/manager/companion_admin.rs` | 新增 `handle_upload_file()` 方法 |
| **修改** | `src/runtime.rs` | 新增 `EmbeddedGatewayRuntime` + `start_embedded_gateway_runtime()` |
| **修改** | `src/runtime/bootstrap.rs` | 初始化 `FileManager` 并传入 `build_agent_loop()` |
| **修改** | `src/runtime/task_runtime.rs` | 新增 `start_embedded_runtime_tasks()`、`spawn_embedded_server_runtime()`、`ServerRuntime` 枚举 |
| **修改** | `src/server.rs` | `build_app()` → `build_router()`（pub）；新增 `run_server_with_listener()`；`/api/files/upload` 路由（50MB 限制） |
| **修改** | `src/state.rs` | `ManagerCommand` 新增 `UploadFile` 变体；新增 `FileUploadRequest` 结构体 |
| **修改** | `Cargo.toml` | 新增 `agent-diva-files`、`mime_guess` 依赖 |

**新增 HTTP API：**
- `POST /api/files/upload` — multipart 文件上传（50MB 限制），返回 `FileAttachment` JSON
- `ChatRequest.attachments: Option<Vec<String>>` — 聊天时携带附件 file_id 列表

**嵌入式 Gateway 支持：**
- `start_embedded_gateway_runtime()` — 供 GUI（Tauri）嵌入式启动
- `run_server_with_listener()` — 接受外部传入的 `TcpListener`
- `EmbeddedGatewayRuntime::shutdown()` — 优雅关闭

---

### 1.8 根目录变更

| 文件 | 说明 |
|------|------|
| `Cargo.toml` | workspace members 新增 `agent-diva-files`；版本 → 0.4.9；tokio features 新增 `io-util`；新增 `agent-diva-files`、`sqlx` workspace 依赖 |
| `Cargo.lock` | +656 行（新增 sqlx、sha2、prost 等依赖树） |
| `.gitignore` | +48 行（新增 debug artifacts 忽略规则） |
| `CLAUDE.md` | +29 行（更新项目文档） |
| `scripts/package-linux.sh` | 微调版本号 |
| `scripts/package-macos.sh` | 微调版本号 |
| `scripts/root-debug-artifacts/` | 新增调试脚本（5 个文件） |

---

## 二、新功能模块详解

### 2.1 Security 模块（agent-diva-core/src/security/）

**架构：**
```
SecurityPolicy
├── PathValidator     — 6 层路径安全校验
│   ├── Layer 1: null 字节检测
│   ├── Layer 2: ../ 遍历检测
│   ├── Layer 3: URL 编码遍历（%2f、%5c）
│   ├── Layer 4: ~ 扩展检测
│   ├── Layer 5: 绝对路径检测
│   └── Layer 6: 禁止前缀匹配（/etc, ~/.ssh, ~/.aws 等）
├── ActionTracker     — 滑动窗口速率限制
│   └── max_actions_per_hour（默认 100）
└── SecurityConfig    — 预设级别配置
    ├── Permissive    — 开发模式，不限制
    ├── Standard      — 默认，workspace_only + 100/h
    ├── Strict        — 50/h + 禁止扩展
    └── Paranoid      — 只读模式，20/h
```

**集成方式：**
- 替代了原来 `Option<PathBuf> allowed_dir` 的简单模式
- `ReadFileTool::new(security)` / `WriteFileTool::new(security)` 统一注入
- 在 `agent_loop.rs`、`loop_tools.rs`、`subagent.rs` 三处同步替换

### 2.2 agent-diva-files 模块

**数据流：**
```
上传: Channel → FileManager::store(bytes, metadata)
      → SHA256 计算 → 去重检查 → 物理存储 → SQLite 索引 → FileHandle

读取: FileManager::get(file_id)
      → SQLite 查询 → FileHandle → FileManager::read(handle) → bytes

删除: FileManager::release(handle)
      → ref_count-- → ref_count=0 时软删除 → 定期清理
```

**SQLite Schema（推断）：**
```sql
CREATE TABLE files (
    id TEXT PRIMARY KEY,          -- "sha256:<hash>"
    path TEXT NOT NULL,           -- "ab/c123..."
    size INTEGER NOT NULL,
    ref_count INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    last_accessed_at TEXT,
    deleted_at TEXT,
    deleted_by TEXT,
    metadata_json TEXT NOT NULL   -- FileMetadata 序列化
);
```

### 2.3 Ollama Provider

**请求流程：**
```
chat() → POST {base_url}/api/chat → 解析 ChatResponse → LLMResponse
chat_stream() → POST {base_url}/api/chat (stream=true)
             → SSE 逐 chunk → OllamaStreamChunk → LLMStreamEvent
             → mpsc::channel → ProviderEventStream
```

**Tool Calling 处理：**
- 非流式：`ChatResponse.message.tool_calls` → `ToolCallRequest`
- 流式：`OllamaStreamChunk.message.tool_calls` → `ToolCallDelta` 事件
- Tool ID 缺失时自动生成 UUID

---

## 三、与 pro 分支的冲突风险项

### 🔴 高风险

| 项目 | 文件 | 原因 |
|------|------|------|
| **AgentLoop 构造函数签名** | `agent-diva-agent/src/agent_loop.rs` | `new()` 和 `with_tools()` 从同步 → async，返回类型从 `Self` → `Result<Self>`。pro 分支若已有改动调用点，合并冲突不可避免 |
| **文件工具构造方式** | `agent-diva-agent/src/agent_loop/loop_tools.rs` | `ReadFileTool::new(allowed_dir)` → `ReadFileTool::new(security)`。pro 分支的 security policies commit（8224c6a）可能已修改相同区域 |
| **lib.rs 模块导出** | `agent-diva-core/src/lib.rs` | main 新增 `pub mod attachment` + `pub mod security`。pro 分支 8224c6a 也可能修改了此文件（"security policies" 标题） |

### 🟡 中风险

| 项目 | 文件 | 原因 |
|------|------|------|
| **默认端口变更** | `agent-diva-core/src/config/schema.rs` | `default_port()` 从 18790 → 3000。pro 分支可能有自己的端口设置 |
| **Manager 结构体** | `agent-diva-manager/src/manager.rs` | 新增 `file_manager` 字段。pro 分支的 manager 重构（925c02b）可能已修改构造函数 |
| **server.rs 路由** | `agent-diva-manager/src/server.rs` | `build_app()` → `build_router()` 重命名 + 新增路由。pro 分支可能已有路由修改 |
| **Cargo.toml 依赖** | 根 `Cargo.toml` | 新增 `agent-diva-files`、`sqlx` workspace 依赖。版本号对齐（都已是 0.4.9） |
| **state.rs 枚举** | `agent-diva-manager/src/state.rs` | `ManagerCommand` 新增 `UploadFile` 变体。pro 分支可能已有自己的变体新增 |

### 🟢 低风险

| 项目 | 文件 | 原因 |
|------|------|------|
| **agent-diva-files** | 整个 crate | 全新 crate，无冲突可能，只需加入 workspace |
| **Ollama provider** | `agent-diva-providers/src/ollama.rs` | 新增文件，无冲突 |
| **QQ/飞书 channel** | `agent-diva-channels/src/qq.rs`, `feishu.rs` | pro 分支不太可能修改这些 channel |
| **IRC 微调** | `agent-diva-channels/src/irc.rs` | match guard 简化，小改动 |
| **附件处理** | `agent-diva-agent/src/agent_loop/loop_turn.rs` | 新增 `load_attachment_contents()` 方法，纯新增代码 |

---

## 四、合并顺序建议

### 阶段一：基础设施（无冲突）

```
1. agent-diva-files（全新 crate）
   → git checkout main -- agent-diva-files/
   → 在根 Cargo.toml 的 [workspace.dependencies] 添加 agent-diva-files 和 sqlx
   → 在 [workspace] members 添加 "agent-diva-files"
```

### 阶段二：核心依赖（低风险）

```
2. agent-diva-core
   → 先合 attachment.rs（纯新增）
   → 再合 security/ 目录（纯新增）
   → 修改 lib.rs（注意与 pro 的 security policies commit 冲突）
   → 修改 Cargo.toml（添加 parking_lot、agent-diva-files 依赖）
   → 修改 config/schema.rs（FeishuConfig.port + default_port 变更）
```

### 阶段三：Provider（低风险）

```
3. agent-diva-providers
   → 新增 ollama.rs + 测试文件（纯新增）
   → Cargo.toml 版本号
```

### 阶段四：Agent 逻辑（高风险，需手动解决）

```
4. agent-diva-agent
   → Cargo.toml（新增 agent-diva-files、dirs 依赖）
   → agent_loop.rs（async 化 + file_manager 字段）⚠️ 与 pro 的 8224c6a 可能冲突
   → loop_tools.rs（SecurityPolicy 替换）⚠️ 同上
   → loop_turn.rs（附件内联逻辑，纯新增）
   → subagent.rs（SecurityPolicy 替换）
```

### 阶段五：Manager（中风险）

```
5. agent-diva-manager
   → 先合 file_service.rs（纯新增）
   → state.rs（新增 FileUploadRequest + UploadFile 变体）
   → handlers.rs（新增 upload_file_handler + ChatRequest.attachments）
   → manager.rs + companion_admin.rs（file_manager 字段 + handler）
   → server.rs（build_router 重命名 + 新路由）⚠️ 注意与 pro 的路由修改
   → runtime/（EmbeddedGatewayRuntime + bootstrap 文件管理器初始化）
   → lib.rs（新导出）
   → Cargo.toml
```

### 阶段六：CLI 和 Channel（低-中风险）

```
6. agent-diva-cli
   → main.rs + chat_commands.rs（FileManager 初始化 + async 化）

7. agent-diva-channels
   → feishu.rs（protobuf 重写）
   → qq.rs（连接管理重写）
   → irc.rs（微调）
   → dingtalk.rs（版本号）
   → Cargo.toml（prost、rustls-tls、openssl）
```

### 阶段七：根目录和验证

```
8. 根 Cargo.toml（确保所有 workspace 依赖和 members 正确）
9. Cargo.lock（cargo update 重新生成）
10. scripts/、.gitignore、CLAUDE.md
11. cargo build --all && cargo test --all && cargo clippy --all
```

---

## 五、合并策略建议

### 推荐：分 crate 逐个 cherry-pick

```bash
# 1. 创建合并分支
git checkout agent-diva-pro
git merge -b merge-main-backend

# 2. 按阶段一到七的顺序逐 crate 合并
# 每合一个 crate，立即 cargo build 验证编译

# 3. 对于高冲突文件，使用 --no-commit 手动解决
git merge main -- agent-diva-files/ --no-commit
# 手动解决冲突后
git commit -m "merge: add agent-diva-files crate from main"
```

### 关键注意事项

1. **版本号一致性**：两个分支都已是 0.4.9，无需额外处理
2. **async 构造函数**：`AgentLoop::new()` 和 `with_tools()` 的 async 化会影响所有调用方（CLI、Manager、GUI），需逐个检查
3. **SecurityPolicy 替换**：pro 分支的 8224c6a（"security policies"）可能已引入了不同的安全策略实现，需对比后再决定是否替换或合并
4. **默认端口 3000**：如果 pro 分支依赖 18790 端口，需要确认是否接受变更或保留原值
5. **sqlx 依赖**：新增的 SQLite 依赖可能影响交叉编译（特别是 Windows → Linux 场景）
6. **openssl vendored**：channels Cargo.toml 中非 Windows 平台添加了 `openssl = { features = ["vendored"] }`，这对 Linux 构建环境有影响

---

## 六、变更统计摘要

| Crate | 新增文件 | 修改文件 | 新增行数 | 删除行数 |
|-------|---------|---------|---------|---------|
| agent-diva-core | 7 | 3 | +1,307 | -6 |
| agent-diva-files | 14 | 0 | +5,000+ | 0 |
| agent-diva-providers | 3 | 1 | +647 | -2 |
| agent-diva-channels | 1 | 5 | +1,462 | -301 |
| agent-diva-agent | 0 | 5 | +218 | -23 |
| agent-diva-cli | 0 | 3 | +47 | -11 |
| agent-diva-manager | 1 | 10 | +364 | -30 |
| 根目录 | 7 | 5 | +1,204 | -41 |
| **合计** | **33** | **32** | **~10,250** | **~414** |
