# Agent-Diva 现有架构分析报告

## 执行摘要

本文档深入分析 agent-diva 的现有架构，识别与 Hermes 自学习机制集成的关键接入点，并评估需要重构的模块。

**关键发现**：
- agent-diva 采用模块化 Cargo workspace 架构，便于集成新功能
- 现有的记忆系统较为简单（MEMORY.md + HISTORY.md），需要升级为 Hermes 的多层存储架构
- agent loop 已有良好的扩展点，可以集成反馈收集和上下文压缩
- 消息总线（Message Bus）提供了解耦的通信机制，适合集成会话管理

---

## 1. 整体架构概览

### 1.1 Cargo Workspace 结构

```
agent-diva/
├── agent-diva-core          # 核心基础设施
├── agent-diva-agent         # Agent 循环和上下文构建
├── agent-diva-providers     # LLM 提供者
├── agent-diva-channels      # 聊天平台集成
├── agent-diva-tools         # 工具系统
├── agent-diva-cli           # CLI 入口
├── agent-diva-service       # Windows 服务
├── agent-diva-migration     # 数据迁移
├── agent-diva-manager       # 本地网关
├── agent-diva-neuron        # GUI 支持库
└── agent-diva-gui           # Tauri 桌面应用
```

### 1.2 数据流架构

**当前数据流**：

```
Channel Handler (Telegram/Discord/etc.)
    ↓
Message Bus (Inbound Queue)
    ↓
Agent Loop
    ├─ Context Builder (组装系统提示)
    │   ├─ MEMORY.md (长期记忆)
    │   ├─ HISTORY.md (会话历史)
    │   └─ Skills (技能定义)
    ├─ LLM Provider (API 调用)
    └─ Tool Execution (工具调用)
    ↓
Message Bus (Outbound Queue)
    ↓
Channel Handler (响应)
```

**存储层**：
- 会话数据：JSONL 文件（`~/.agent-diva/sessions/`）
- 记忆数据：Markdown 文件（`MEMORY.md`, `HISTORY.md`）
- 配置数据：JSON 文件（`~/.agent-diva/config.json`）

---

## 2. 核心模块分析

### 2.1 agent-diva-core

**职责**：提供核心基础设施，包括消息总线、配置加载、会话管理、记忆系统、错误处理。

#### 2.1.1 消息总线（Message Bus）

**文件**：`agent-diva-core/src/bus/`

**架构**：

```rust
// bus/events.rs
pub enum AgentBusEvent {
    Inbound(InboundMessage),
    Outbound(OutboundMessage),
    Agent(AgentEvent),
}

pub struct InboundMessage {
    pub session_id: String,
    pub channel: String,
    pub user_id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

pub struct OutboundMessage {
    pub session_id: String,
    pub channel: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

// bus/queue.rs
pub struct MessageBus {
    inbound: Arc<Mutex<VecDeque<InboundMessage>>>,
    outbound: Arc<Mutex<VecDeque<OutboundMessage>>>,
}
```

**特点**：
- 双队列设计（inbound + outbound）
- 解耦 Channel Handler 和 Agent Loop
- 使用 `Arc<Mutex<VecDeque>>` 实现线程安全

**Hermes 集成点**：
- ✅ 可以在消息入队/出队时记录到 SessionDB
- ✅ 可以在此处收集会话元数据（timestamp, channel, user_id）
- ⚠️ 需要添加工具调用统计的事件类型

#### 2.1.2 会话管理（Session Manager）

**文件**：`agent-diva-core/src/session/`（推测，未在当前代码中找到）

**当前实现**：
- 会话数据持久化到 JSONL 文件
- 每个会话一个文件：`~/.agent-diva/sessions/{session_id}.jsonl`
- 追加式写入，无索引

**问题**：
- ❌ 无跨会话搜索能力
- ❌ 无全文搜索索引
- ❌ 无会话元数据统计（token 使用、成本、工具调用）
- ❌ 无压缩链追踪

**Hermes 集成方案**：
- 🔄 替换为 SQLite + WAL + FTS5
- 🔄 添加 sessions 表和 messages 表
- 🔄 实现压缩链追踪（parent_session_id）

#### 2.1.3 记忆系统（Memory System）

**文件**：`agent-diva-core/src/memory/`

**当前实现**：

```rust
// memory/storage.rs
pub struct Memory {
    pub content: String,           // Markdown 内容
    pub updated_at: DateTime<Utc>,
    pub version: u64,              // 版本号（冲突检测）
}

// memory/manager.rs
pub struct MemoryManager {
    memory_path: PathBuf,          // MEMORY.md 路径
    history_path: PathBuf,         // HISTORY.md 路径
}

impl MemoryManager {
    pub async fn load_memory(&self) -> Result<Memory>;
    pub async fn save_memory(&self, memory: &Memory) -> Result<()>;
    pub async fn append_history(&self, entry: &str) -> Result<()>;
}
```

**特点**：
- 简单的文件读写
- 版本号用于冲突检测
- HISTORY.md 是追加式日志

**问题**：
- ❌ 无记忆检索能力（全量注入到上下文）
- ❌ 无记忆权重分级
- ❌ 无记忆衰减机制
- ❌ 无事实反馈系统
- ❌ 无多提供者支持

**Hermes 集成方案**：
- 🔄 实现 MemoryProvider 抽象接口
- 🔄 实现 BuiltinMemoryProvider（保留现有 MEMORY.md）
- 🔄 实现 HolographicMemoryProvider（事实存储）
- 🔄 实现 MemoryManager 协调器

---

### 2.2 agent-diva-agent

**职责**：实现 Agent 循环、上下文构建、技能加载、子代理管理。

#### 2.2.1 Agent Loop

**文件**：`agent-diva-agent/src/agent_loop.rs`

**核心结构**：

```rust
pub struct AgentLoop {
    session_id: String,
    context_builder: ContextBuilder,
    provider: Arc<dyn Provider>,
    tool_registry: Arc<ToolRegistry>,
    message_bus: Arc<MessageBus>,
}

impl AgentLoop {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            // 1. 从 Message Bus 获取消息
            let msg = self.message_bus.pop_inbound().await?;
            
            // 2. 构建上下文
            let context = self.context_builder.build(&msg).await?;
            
            // 3. 调用 LLM
            let response = self.provider.complete(&context).await?;
            
            // 4. 执行工具调用
            if let Some(tool_calls) = response.tool_calls {
                for call in tool_calls {
                    self.execute_tool(&call).await?;
                }
            }
            
            // 5. 发送响应
            self.message_bus.push_outbound(response).await?;
        }
    }
}
```

**Hermes 集成点**：
- ✅ 在步骤 1 后：记录 user 消息到 SessionDB
- ✅ 在步骤 3 后：记录 assistant 消息到 SessionDB
- ✅ 在步骤 4 中：记录工具调用统计
- ✅ 在步骤 5 后：触发上下文压缩检查
- ✅ 在循环结束时：触发 on_session_end 钩子

#### 2.2.2 Context Builder

**文件**：`agent-diva-agent/src/context.rs`

**核心逻辑**：

```rust
pub struct ContextBuilder {
    memory_manager: Arc<MemoryManager>,
    skill_loader: Arc<SkillLoader>,
    session_manager: Arc<SessionManager>,
}

impl ContextBuilder {
    pub async fn build(&self, msg: &InboundMessage) -> Result<Context> {
        let mut context = Context::new();
        
        // 1. 加载系统提示
        context.add_system_prompt(self.build_system_prompt().await?);
        
        // 2. 加载记忆
        let memory = self.memory_manager.load_memory().await?;
        context.add_memory(&memory.content);
        
        // 3. 加载历史
        let history = self.session_manager.load_history(&msg.session_id).await?;
        context.add_history(history);
        
        // 4. 加载技能
        let skills = self.skill_loader.load_skills().await?;
        context.add_skills(skills);
        
        // 5. 添加用户消息
        context.add_user_message(&msg.content);
        
        Ok(context)
    }
}
```

**Hermes 集成点**：
- ✅ 在步骤 2：调用 MemoryManager.prefetch_all()
- ✅ 在步骤 3：从 SessionDB 加载历史（而非 JSONL）
- ✅ 在步骤 3：检查是否需要上下文压缩
- ✅ 在步骤 4：添加记忆提供者的工具模式

#### 2.2.3 Consolidation（整合）

**文件**：`agent-diva-agent/src/consolidation.rs`

**当前实现**：未找到具体实现，可能尚未开发。

**Hermes 对应功能**：
- 上下文压缩（Context Compressor）
- 记忆整合（Memory Consolidation）

**需要实现**：
- 🔄 实现上下文压缩触发逻辑
- 🔄 实现 LLM 摘要生成
- 🔄 实现工具输出剪枝
- 🔄 实现保护头尾消息策略

---

### 2.3 agent-diva-providers

**职责**：LLM 提供者抽象和实现（OpenRouter, Anthropic, OpenAI, DeepSeek, Groq, Gemini）。

**文件**：`agent-diva-providers/src/`

**Provider Trait**：

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, context: &Context) -> Result<Response>;
    async fn stream(&self, context: &Context) -> Result<ResponseStream>;
    fn name(&self) -> &str;
    fn supports_tools(&self) -> bool;
}
```

**Hermes 集成点**：
- ✅ 在 complete() 返回后：记录 token 使用统计
- ✅ 在 complete() 返回后：计算成本估算
- ✅ 在 stream() 中：实时更新 token 计数
- ⚠️ 需要添加 reasoning 字段支持（Claude 的思维过程）

---

### 2.4 agent-diva-tools

**职责**：工具系统，包括工具注册表、工具执行、内置工具实现。

**文件**：`agent-diva-tools/src/`

**Tool Trait**：

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> Result<String>;
}
```

**Hermes 集成点**：
- ✅ 在 execute() 前后：记录工具调用统计
- ✅ 在 execute() 失败时：记录错误信息
- ✅ 在 execute() 成功时：记录执行时长
- 🔄 添加记忆提供者工具（fact_feedback, search_facts, etc.）

---

## 3. 关键接入点识别

### 3.1 会话生命周期钩子

**需要添加的钩子**：

```rust
pub trait SessionHooks {
    async fn on_session_start(&self, session_id: &str);
    async fn on_turn_start(&self, turn_number: u32, message: &str);
    async fn on_turn_end(&self, turn_number: u32, response: &str);
    async fn on_tool_call(&self, tool_name: &str, args: &Value, result: &str);
    async fn on_session_end(&self, session_id: &str);
    async fn on_pre_compress(&self, messages: &[Message]) -> String;
}
```

**集成位置**：
- `agent-diva-agent/src/agent_loop.rs` - 在 Agent Loop 的关键点调用钩子
- `agent-diva-core/src/session/hooks.rs` - 定义钩子 trait

### 3.2 记忆系统扩展点

**需要添加的接口**：

```rust
#[async_trait]
pub trait MemoryProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn initialize(&self, session_id: &str) -> Result<()>;
    fn is_available(&self) -> bool;
    fn get_tool_schemas(&self) -> Vec<Value>;
    fn system_prompt_block(&self) -> String;
    async fn prefetch(&self, query: &str, session_id: &str) -> Result<String>;
    async fn sync_turn(&self, user_content: &str, assistant_content: &str) -> Result<()>;
    async fn handle_tool_call(&self, tool_name: &str, args: &Value) -> Result<String>;
    async fn on_session_end(&self, messages: &[Message]) -> Result<()>;
    async fn on_pre_compress(&self, messages: &[Message]) -> Result<String>;
}

pub struct MemoryManager {
    providers: Vec<Box<dyn MemoryProvider>>,
    builtin: BuiltinMemoryProvider,
    external: Option<Box<dyn MemoryProvider>>,
}
```

**集成位置**：
- `agent-diva-core/src/memory/provider.rs` - 定义 MemoryProvider trait
- `agent-diva-core/src/memory/manager.rs` - 重构 MemoryManager
- `agent-diva-core/src/memory/builtin.rs` - 实现 BuiltinMemoryProvider
- `agent-diva-core/src/memory/holographic.rs` - 实现 HolographicMemoryProvider

### 3.3 SessionDB 集成点

**需要添加的模块**：

```rust
// agent-diva-core/src/session/db.rs
pub struct SessionDB {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SessionDB {
    pub async fn new(path: &Path) -> Result<Self>;
    pub async fn save_session(&self, session: &Session) -> Result<()>;
    pub async fn append_message(&self, session_id: &str, message: &Message) -> Result<()>;
    pub async fn load_history(&self, session_id: &str, limit: usize) -> Result<Vec<Message>>;
    pub async fn search_messages(&self, query: &str) -> Result<Vec<Message>>;
    pub async fn get_session_stats(&self, session_id: &str) -> Result<SessionStats>;
}
```

**集成位置**：
- `agent-diva-core/src/session/db.rs` - SessionDB 实现
- `agent-diva-core/src/session/schema.sql` - 数据库模式
- `agent-diva-core/src/session/migration.rs` - 从 JSONL 迁移到 SQLite

---

## 4. 需要重构的模块

### 4.1 高优先级重构

#### 4.1.1 会话管理（Session Manager）

**当前问题**：
- JSONL 文件无索引，无法高效搜索
- 无会话元数据统计
- 无压缩链追踪

**重构方案**：
- 替换为 SQLite + WAL + FTS5
- 实现 SessionDB 模块
- 保留 JSONL 作为备份/导出格式

**工作量**：2-3 周

#### 4.1.2 记忆系统（Memory System）

**当前问题**：
- 简单的文件读写，无检索能力
- 全量注入上下文，无权重分级
- 无多提供者支持

**重构方案**：
- 实现 MemoryProvider 抽象
- 实现 MemoryManager 协调器
- 实现 BuiltinMemoryProvider（保留现有功能）
- 实现 HolographicMemoryProvider（事实存储）

**工作量**：3-4 周

#### 4.1.3 Agent Loop

**当前问题**：
- 无反馈收集点
- 无上下文压缩触发
- 无会话生命周期钩子

**重构方案**：
- 添加 SessionHooks trait
- 在关键点调用钩子
- 集成 MemoryManager.prefetch_all()
- 集成上下文压缩检查

**工作量**：2-3 周

### 4.2 中优先级重构

#### 4.2.1 Context Builder

**当前问题**：
- 硬编码的上下文构建逻辑
- 无动态记忆召回

**重构方案**：
- 集成 MemoryManager.prefetch_all()
- 实现上下文压缩触发
- 添加记忆提供者工具模式

**工作量**：1-2 周

#### 4.2.2 Provider Trait

**当前问题**：
- 无 reasoning 字段支持
- 无 token 使用统计

**重构方案**：
- 添加 reasoning 字段到 Response
- 添加 token 统计到 Response
- 实现成本计算逻辑

**工作量**：1 周

### 4.3 低优先级重构

#### 4.3.1 Tool System

**当前问题**：
- 无工具调用统计
- 无执行时长记录

**重构方案**：
- 在 Tool::execute() 前后记录统计
- 添加工具调用钩子

**工作量**：1 周

---

## 5. 数据迁移策略

### 5.1 JSONL → SQLite 迁移

**迁移步骤**：

1. **创建 SQLite 数据库**
   - 运行 schema.sql 创建表结构
   - 启用 WAL 模式
   - 创建 FTS5 索引

2. **读取 JSONL 文件**
   - 扫描 `~/.agent-diva/sessions/` 目录
   - 解析每个 JSONL 文件

3. **转换数据格式**
   - 提取会话元数据（session_id, started_at, ended_at）
   - 转换消息格式（role, content, tool_calls）
   - 计算 token 统计（如果可用）

4. **写入 SQLite**
   - 插入 sessions 表
   - 插入 messages 表
   - 更新 FTS5 索引

5. **验证迁移**
   - 检查记录数
   - 测试搜索功能
   - 验证数据完整性

**迁移工具**：
- `agent-diva-migration/src/jsonl_to_sqlite.rs`

### 5.2 MEMORY.md 保留策略

**策略**：
- ✅ 保留 MEMORY.md 作为用户可编辑的记忆文件
- ✅ 实现 BuiltinMemoryProvider 读取 MEMORY.md
- ✅ 添加 Holographic 事实存储作为补充
- ✅ 两者通过 MemoryManager 协调

**无需迁移**：MEMORY.md 继续使用，无破坏性变更。

---

## 6. 架构改进建议

### 6.1 分层架构

**建议的新架构**：

```
┌─────────────────────────────────────────┐
│  应用层 (CLI / Gateway / GUI)           │
├─────────────────────────────────────────┤
│  Agent Loop (agent-diva-agent)          │
│  - 反馈收集                              │
│  - 上下文压缩                            │
│  - 会话生命周期钩子                      │
├─────────────────────────────────────────┤
│  记忆管理层 (MemoryManager)             │
│  - BuiltinMemoryProvider (MEMORY.md)    │
│  - HolographicMemoryProvider (事实存储) │
│  - 其他插件 (Honcho, Mem0, etc.)       │
├─────────────────────────────────────────┤
│  持久化层 (SessionDB + 文件系统)        │
│  - SQLite state.db (FTS5 全文搜索)      │
│  - MEMORY.md / USER.md (文件存储)       │
│  - Holographic memory_store.db          │
├─────────────────────────────────────────┤
│  工具层 (Tool Registry + Execution)     │
└─────────────────────────────────────────┘
```

### 6.2 并发模型

**当前**：
- Message Bus 使用 `Arc<Mutex<VecDeque>>`
- 简单的锁机制

**建议**：
- SessionDB 使用 SQLite WAL 模式（多读单写）
- 应用层重试机制（20-150ms 随机抖动）
- 避免长时间持有锁

### 6.3 性能优化

**建议**：
- 实现提示缓存（Anthropic Prompt Caching）
- 实现上下文压缩（50% 上下文窗口触发）
- 实现 FTS5 全文搜索（快速跨会话搜索）
- 实现工具输出剪枝（廉价预处理）

---

## 7. 总结

### 7.1 关键发现

1. **架构兼容性**：agent-diva 的模块化架构非常适合集成 Hermes 自学习机制
2. **主要差距**：会话管理和记忆系统需要升级
3. **集成点清晰**：Agent Loop 和 Context Builder 是主要集成点
4. **重构可控**：大部分重构是增量式的，无破坏性变更

### 7.2 下一步行动

1. **Phase 1**：实现 SessionDB（SQLite + WAL + FTS5）
2. **Phase 2**：实现 MemoryProvider 抽象和 MemoryManager
3. **Phase 3**：重构 Agent Loop 添加钩子和反馈收集
4. **Phase 4**：实现上下文压缩和记忆整合
5. **Phase 5**：实现事实反馈系统和自学习能力

### 7.3 风险评估

| 风险 | 等级 | 缓解策略 |
|------|------|---------|
| 数据迁移失败 | 🟡 中 | 保留 JSONL 备份，实现回滚机制 |
| 性能下降 | 🟡 中 | 使用 WAL 模式，实现索引优化 |
| 并发冲突 | 🔴 高 | 统一写入路径，使用 SQLite 管理并发 |
| 破坏现有功能 | 🟢 低 | 增量式重构，保留现有接口 |

---

**文档版本**：v1.0  
**创建日期**：2026-04-05  
**作者**：Agent Diva Team  
**状态**：草稿
