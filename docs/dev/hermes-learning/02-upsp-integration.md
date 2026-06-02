# UPSP 与 Hermes 集成方案

> **版本**: v0.1.0-draft  
> **日期**: 2026-04-05

---

## 1. 兼容性分析总结

### 1.1 协同点（高度兼容）

| 维度 | UPSP | Hermes | 协同方案 |
|------|------|--------|---------|
| **记忆存储** | 七文件体系（STM.md + LTM.md） | MemoryProvider 抽象 + HolographicMemoryProvider | UPSP 作为 MemoryProvider 的一种实现 |
| **检索能力** | 混合检索（关键词+语义+时间）+ SQLite 索引 | SessionDB（SQLite + FTS5） | 共享同一套索引基础设施 |
| **会话管理** | 节律点机制 + history.json | SessionDB + 会话生命周期钩子 | history.json 由 SessionDB 提供 |
| **上下文构建** | ContextLoader + 按权重召回 | MemoryLoader + 主动召回（3~7 条） | 融合为统一的上下文加载器 |

### 1.2 潜在冲突点

#### 冲突 1：记忆存储格式

**问题**：
- UPSP：完全替代 MEMORY.md，使用七文件体系
- Hermes：保留 MEMORY.md 作为 BuiltinMemoryProvider

**解决方案**：
```
UPSP 的 STM.md/LTM.md 替代 MEMORY.md
↓
BuiltinMemoryProvider 读取 UPSP 文件而非 MEMORY.md
↓
Phase 1-2 保留双写模式（过渡期）
↓
Phase 3 完全迁移到 UPSP
```

#### 冲突 2：consolidation 触发机制

**问题**：
- UPSP：节律点（每 32 轮触发）
- Hermes：上下文压缩（50% 上下文窗口触发）

**解决方案 - 统一触发器**：

```rust
// agent-diva-agent/src/consolidation/trigger.rs
pub struct ConsolidationTrigger {
    rhythm_point_interval: usize,  // 32 轮
    context_window_threshold: f32, // 0.5 (50%)
}

impl ConsolidationTrigger {
    pub fn should_trigger(&self, state: &AgentState) -> ConsolidationReason {
        // 检查节律点
        if state.turn_count % self.rhythm_point_interval == 0 {
            return ConsolidationReason::RhythmPoint;
        }
        
        // 检查上下文窗口
        let usage = state.context_tokens as f32 / state.max_context_tokens as f32;
        if usage >= self.context_window_threshold {
            return ConsolidationReason::ContextWindow;
        }
        
        ConsolidationReason::None
    }
}

pub enum ConsolidationReason {
    None,
    RhythmPoint,      // UPSP 节律点 → 记忆整合（STM → LTM）
    ContextWindow,    // Hermes 压缩 → 会话历史摘要（messages → summary）
}
```

**职责分工**：
- **节律点**：负责记忆整合（STM → LTM）、关系更新、状态结算
- **上下文压缩**：负责会话历史摘要（messages → summary message）

#### 冲突 3：索引层职责

**问题**：
- UPSP Phase 2：agent-diva 侧自建索引层（SQLite + 向量数据库）
- Hermes：SessionDB（SQLite + FTS5）

**解决方案 - 统一数据库（brain.db）**：

```
brain.db (SQLite + WAL)
├── sessions 表（Hermes SessionDB）
│   ├── session_id, channel, user_id, created_at, updated_at
│   └── last_consolidated, total_tokens, total_cost
├── messages 表（Hermes SessionDB）
│   ├── id, session_id, role, content, tool_calls
│   ├── timestamp, tokens, reasoning_content
│   └── FTS5 索引（content 全文搜索）
├── memories 表（UPSP 索引层）
│   ├── id, persona_id, memory_type (STM/LTM)
│   ├── content, weight, created_at, last_accessed
│   └── FTS5 索引（content 全文搜索）
├── relations 表（UPSP 关系管理）
│   ├── id, persona_id, entity_name, resonance
│   └── last_updated
└── embeddings 表（向量检索，可选）
    ├── id, source_type (message/memory), source_id
    └── embedding BLOB
```

**分层查询**：
```rust
// SessionDB 负责会话历史查询
let history = session_db.load_history(session_id, limit).await?;

// MemoryStore 负责长期记忆查询
let memories = memory_store.search_memories(query, limit).await?;

// 共享同一数据库连接
let conn = Arc::new(Mutex::new(Connection::open("brain.db")?));
```

#### 冲突 4：MemoryProvider 抽象

**问题**：
- UPSP：upsp-rs 仅提供序列化/反序列化，不提供 MemoryProvider 接口
- Hermes：定义 MemoryProvider trait（initialize、prefetch、sync_turn、handle_tool_call、on_session_end）

**解决方案 - 适配器模式**：

```rust
// agent-diva-core/src/memory/upsp_adapter.rs
pub struct UpspMemoryProvider {
    persona: Arc<Mutex<upsp_rs::Persona>>,
    store: Arc<upsp_rs::FilesystemStore>,
}

#[async_trait]
impl MemoryProvider for UpspMemoryProvider {
    fn name(&self) -> &str {
        "upsp"
    }
    
    async fn initialize(&self, session_id: &str) -> Result<()> {
        let mut persona = self.persona.lock().await;
        persona.state.session_id = session_id.to_string();
        persona.state.turn_count = 0;
        Ok(())
    }
    
    fn is_available(&self) -> bool {
        self.store.exists()
    }
    
    fn get_tool_schemas(&self) -> Vec<Value> {
        // UPSP 不提供工具，返回空
        vec![]
    }
    
    fn system_prompt_block(&self) -> String {
        let persona = self.persona.blocking_lock();
        
        // 从 core.md 和 state.json 构建系统提示
        format!(
            "# Identity\n{}\n\n# Current State\n{}",
            persona.core.self_description,
            serde_json::to_string_pretty(&persona.state).unwrap()
        )
    }
    
    async fn prefetch(&self, query: &str, session_id: &str) -> Result<String> {
        let persona = self.persona.lock().await;
        
        // 从 STM 和 LTM 召回相关记忆
        let stm_memories = persona.stm.recall(query, 3)?;
        let ltm_memories = persona.ltm.recall(query, 5)?;
        
        // 按权重格式化
        let mut context = String::new();
        for mem in stm_memories {
            context.push_str(&format!("[F] {}\n", mem.content));
        }
        for mem in ltm_memories {
            let prefix = match mem.weight {
                5 => "[F]",
                4 | 3 => "[S]",
                2 | 1 => "[A]",
                _ => "[?]",
            };
            context.push_str(&format!("{} {}\n", prefix, mem.content));
        }
        
        Ok(context)
    }
    
    async fn sync_turn(&self, user_content: &str, assistant_content: &str) -> Result<()> {
        let mut persona = self.persona.lock().await;
        
        // 更新 turn_count
        persona.state.turn_count += 1;
        
        // 添加到 STM
        persona.stm.add_entry(MemoryEntry {
            content: format!("User: {}\nAssistant: {}", user_content, assistant_content),
            weight: 5,  // Full memory
            timestamp: Utc::now(),
        })?;
        
        // 保存 state.json
        self.store.save_state(&persona.state).await?;
        
        Ok(())
    }
    
    async fn handle_tool_call(&self, tool_name: &str, args: &Value) -> Result<String> {
        // UPSP 不处理工具调用
        Err(anyhow!("UPSP does not handle tool calls"))
    }
    
    async fn on_session_end(&self, messages: &[Message]) -> Result<()> {
        let mut persona = self.persona.lock().await;
        
        // 检查是否到达节律点
        if persona.state.turn_count % 32 == 0 {
            // 执行节律点整合
            self.execute_rhythm_point(&mut persona, messages).await?;
        }
        
        Ok(())
    }
    
    async fn on_pre_compress(&self, messages: &[Message]) -> Result<String> {
        let persona = self.persona.lock().await;
        
        // 从最近的对话中提取摘要
        let summary = self.summarize_recent_turns(&persona, messages).await?;
        
        Ok(summary)
    }
}

impl UpspMemoryProvider {
    async fn execute_rhythm_point(&self, persona: &mut Persona, messages: &[Message]) -> Result<()> {
        // 1. 从 history.json 提取最近 4 轮
        let recent_turns = &messages[messages.len().saturating_sub(4)..];
        
        // 2. 写入 STM 快照区
        for turn in recent_turns {
            persona.stm.add_snapshot(turn)?;
        }
        
        // 3. STM → LTM 整合
        let consolidated = persona.stm.consolidate()?;
        for entry in consolidated {
            persona.ltm.add_entry(entry)?;
        }
        
        // 4. 关系更新
        persona.relations.update_resonance()?;
        
        // 5. 状态结算
        persona.state.workhood_index.recalculate()?;
        
        // 6. 保存所有文件
        self.store.save_persona(persona).await?;
        
        Ok(())
    }
}
```

---

## 2. 融合架构设计

### 2.1 分层架构

```
┌─────────────────────────────────────────────────────────────┐
│  应用层 (Agent Loop + Context Builder)                      │
│  - 会话生命周期钩子（Hermes）                                │
│  - 节律点触发器（UPSP）                                      │
│  - RL 训练编排（Hermes）                                     │
│  - 技能自动创建（Hermes）                                    │
├─────────────────────────────────────────────────────────────┤
│  记忆管理层 (MemoryManager)                                 │
│  - UpspMemoryProvider（UPSP 适配器）                        │
│  - HolographicMemoryProvider（Hermes 事实存储）             │
│  - SkillMemoryProvider（技能系统）                          │
│  - 协调器：prefetch_all, sync_all, extract_all             │
├─────────────────────────────────────────────────────────────┤
│  存储层                                                      │
│  - UPSP 七文件（core.md, state.json, STM.md, LTM.md, etc.）│
│  - brain.db（统一 SQLite 数据库）                           │
│    ├── sessions 表（Hermes SessionDB）                      │
│    ├── messages 表（Hermes SessionDB）                      │
│    ├── memories 表（UPSP 索引层）                           │
│    ├── relations 表（UPSP 关系管理）                        │
│    └── embeddings 表（向量检索，可选）                      │
│  - Trajectory Store（训练数据）                             │
│  - Skills 目录（技能文件）                                  │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 职责划分

#### UPSP 负责

1. **长期记忆（LTM）**：
   - 记忆归档和索引
   - 权重分级（5→[F], 4/3→[S], 2/1→[A]）
   - 记忆召回策略

2. **身份管理（core.md）**：
   - 核心六轴（长期认知风格）
   - 自述和模型戳
   - 身份常量

3. **关系管理（relation.md）**：
   - 共振度计算
   - 关系域维护
   - 关系演化

4. **节律点机制**：
   - 每 32 轮触发
   - STM → LTM 整合
   - 关系更新和状态结算

#### Hermes 负责

1. **短期记忆（SessionDB）**：
   - 会话历史持久化
   - 跨会话搜索（FTS5）
   - Token 统计和成本追踪

2. **事实反馈（HolographicMemoryProvider）**：
   - 事实存储和检索
   - 事实验证和更新
   - 事实关联

3. **上下文压缩**：
   - 50% 窗口触发
   - 会话历史摘要
   - 工具输出剪枝

4. **技能系统**：
   - 自动创建触发
   - 技能 CRUD 操作
   - 安全扫描

5. **RL 训练闭环**：
   - Trajectory 保存和压缩
   - 训练编排
   - 模型改进

### 2.3 数据流

**完整的学习闭环**：

```
用户消息
    ↓
[应用层] Agent Loop 接收
    ↓
[记忆管理层] MemoryManager.prefetch_all()
    ├─ UpspMemoryProvider.prefetch() → STM/LTM 召回
    ├─ HolographicMemoryProvider.prefetch() → 事实召回
    └─ SkillMemoryProvider.prefetch() → 技能召回
    ↓
[应用层] Context Builder 构建上下文
    ↓
[应用层] LLM Provider 调用
    ↓
[应用层] Tool Execution
    ↓
[存储层] Trajectory Store 保存
    ↓
[记忆管理层] MemoryManager.sync_all()
    ├─ UpspMemoryProvider.sync_turn() → 更新 STM + state.json
    ├─ HolographicMemoryProvider.sync_turn() → 更新事实
    └─ SkillMemoryProvider.sync_turn() → 检查技能创建触发
    ↓
[存储层] SessionDB 保存消息
    ↓
[应用层] ConsolidationTrigger 检查
    ├─ 节律点？ → UpspMemoryProvider.on_session_end() → 节律点整合
    └─ 上下文窗口？ → 上下文压缩 → 会话历史摘要
    ↓
[存储层] Trajectory Compressor 压缩
    ↓
[应用层] RL Trainer 训练（可选）
    ↓
模型改进
```

---

## 3. 迁移策略

### 3.1 分阶段迁移

#### Phase 1：并行运行（2-4 周）

**目标**：UPSP-RS 作为可选 feature，现有系统继续工作

**实施**：
```toml
# Cargo.toml
[features]
upsp = ["upsp-rs"]

[dependencies]
upsp-rs = { version = "0.1", optional = true }
```

```rust
// agent-diva-core/src/memory/manager.rs
pub struct MemoryManager {
    builtin: BuiltinMemoryProvider,  // 读取 MEMORY.md
    #[cfg(feature = "upsp")]
    upsp: Option<UpspMemoryProvider>,  // 读取 UPSP 七文件
}
```

**验证**：
- 两套系统并行运行
- 对比输出一致性
- 性能基准测试

#### Phase 2：双写模式（2-3 周）

**目标**：consolidation 同时写入 MEMORY.md 和 UPSP 七文件

**实施**：
```rust
// agent-diva-agent/src/consolidation/mod.rs
pub async fn consolidate(&self, messages: &[Message]) -> Result<()> {
    // 1. 调用 LLM 生成摘要
    let summary = self.generate_summary(messages).await?;
    
    // 2. 写入 MEMORY.md（旧系统）
    self.builtin.save_memory(&summary).await?;
    
    // 3. 写入 UPSP 七文件（新系统）
    #[cfg(feature = "upsp")]
    if let Some(upsp) = &self.upsp {
        upsp.sync_turn("", &summary).await?;
    }
    
    Ok(())
}
```

**验证**：
- 两套系统数据一致
- 迁移工具可用
- 回滚机制有效

#### Phase 3：完全迁移（1-2 周）

**目标**：UPSP 成为默认且唯一记忆模型，废弃 MEMORY.md/HISTORY.md

**实施**：
```rust
// agent-diva-core/src/memory/manager.rs
pub struct MemoryManager {
    upsp: UpspMemoryProvider,  // 唯一记忆提供者
    holographic: Option<HolographicMemoryProvider>,  // 可选事实存储
}
```

**迁移工具**：
```bash
# 从 MEMORY.md 迁移到 UPSP
agent-diva migrate memory-to-upsp \
    --input ~/.agent-diva/memory/MEMORY.md \
    --output ~/.agent-diva/persona/

# 从 JSONL 迁移到 SessionDB
agent-diva migrate sessions-to-db \
    --input ~/.agent-diva/sessions/ \
    --output ~/.agent-diva/brain.db
```

### 3.2 数据迁移工具

**MEMORY.md → UPSP LTM.md**：

```rust
// agent-diva-migration/src/memory_to_upsp.rs
pub async fn migrate_memory_to_upsp(input: &Path, output: &Path) -> Result<()> {
    // 1. 读取 MEMORY.md
    let content = tokio::fs::read_to_string(input).await?;
    
    // 2. 解析为记忆条目
    let entries = parse_memory_markdown(&content)?;
    
    // 3. 转换为 UPSP 格式
    let ltm = LongTermMemory {
        entries: entries.into_iter().map(|e| MemoryEntry {
            content: e.content,
            weight: 4,  // 默认 Summary 权重
            timestamp: e.timestamp.unwrap_or_else(Utc::now),
        }).collect(),
    };
    
    // 4. 写入 LTM.md
    let ltm_path = output.join("LTM.md");
    tokio::fs::write(ltm_path, ltm.to_markdown()?).await?;
    
    Ok(())
}
```

**JSONL → SessionDB**：

```rust
// agent-diva-migration/src/sessions_to_db.rs
pub async fn migrate_sessions_to_db(input: &Path, output: &Path) -> Result<()> {
    let db = SessionDB::new(output).await?;
    
    // 遍历所有 JSONL 文件
    for entry in std::fs::read_dir(input)? {
        let path = entry?.path();
        if path.extension() != Some(OsStr::new("jsonl")) {
            continue;
        }
        
        // 解析 JSONL
        let session = Session::from_jsonl(&path).await?;
        
        // 写入数据库
        db.save_session(&session).await?;
        for message in &session.messages {
            db.append_message(&session.key, message).await?;
        }
    }
    
    Ok(())
}
```

---

## 4. 配置扩展

### 4.1 Cargo.toml

```toml
[workspace]
members = [
    "agent-diva-core",
    "agent-diva-agent",
    # ...
]

[workspace.dependencies]
upsp-rs = { version = "0.1", optional = true }

[features]
default = ["upsp"]
upsp = ["upsp-rs", "agent-diva-core/upsp"]
rl-training = ["agent-diva-core/rl-training"]
```

### 4.2 config.json

```json
{
  "agents": {
    "defaults": {
      "provider": "openrouter",
      "model": "anthropic/claude-sonnet-4"
    },
    "upsp": {
      "enabled": true,
      "rhythm": {
        "max_rounds": 32
      },
      "memory": {
        "stm_max_entries": 100,
        "ltm_max_entries": 1000
      }
    },
    "hermes": {
      "session_db": {
        "path": "~/.agent-diva/brain.db",
        "wal_mode": true
      },
      "consolidation": {
        "context_window_threshold": 0.5,
        "trigger_on_rhythm_point": true
      },
      "skills": {
        "auto_create": true,
        "min_tool_calls": 5,
        "security_scan": true
      },
      "rl_training": {
        "enabled": false,
        "trajectory_dir": "~/.agent-diva/trajectories",
        "compression": {
          "target_max_tokens": 15250,
          "summary_target_tokens": 750
        }
      }
    }
  }
}
```

---

## 5. 测试策略

### 5.1 单元测试

**UpspMemoryProvider 适配器**：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_upsp_adapter_prefetch() {
        let provider = UpspMemoryProvider::new_test().await;
        
        let context = provider.prefetch("test query", "session-1").await.unwrap();
        
        assert!(context.contains("[F]"));  // Full memory
        assert!(context.contains("[S]"));  // Summary memory
    }
    
    #[tokio::test]
    async fn test_upsp_adapter_sync_turn() {
        let provider = UpspMemoryProvider::new_test().await;
        
        provider.sync_turn("user message", "assistant response").await.unwrap();
        
        let persona = provider.persona.lock().await;
        assert_eq!(persona.state.turn_count, 1);
        assert_eq!(persona.stm.entries.len(), 1);
    }
}
```

### 5.2 集成测试

**完整学习闭环**：

```rust
#[tokio::test]
async fn test_full_learning_loop() {
    let agent = AgentLoop::new_test().await;
    
    // 1. 用户交互
    agent.process_message("user message").await.unwrap();
    
    // 2. 验证 Trajectory 保存
    let trajectory = agent.trajectory_store.load("session-1").await.unwrap();
    assert_eq!(trajectory.conversations.len(), 2);  // user + assistant
    
    // 3. 验证 SessionDB 保存
    let history = agent.session_db.load_history("session-1", 10).await.unwrap();
    assert_eq!(history.len(), 2);
    
    // 4. 验证 UPSP 更新
    let persona = agent.memory_manager.upsp.persona.lock().await;
    assert_eq!(persona.state.turn_count, 1);
}
```

### 5.3 性能测试

**基准测试**：

```rust
#[bench]
fn bench_upsp_prefetch(b: &mut Bencher) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let provider = rt.block_on(UpspMemoryProvider::new_test());
    
    b.iter(|| {
        rt.block_on(provider.prefetch("test query", "session-1"))
    });
}
```

---

**文档版本**：v0.1.0-draft  
**最后更新**：2026-04-05
