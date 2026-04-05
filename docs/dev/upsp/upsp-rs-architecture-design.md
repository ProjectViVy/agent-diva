# UPSP-RS 架构设计文档

> **版本**: v0.1.0-draft  
> **日期**: 2026-04-05  
> **范围**: UPSP协议的Rust实现，作为独立crate发布到crates.io，并深度集成到agent-diva

---

## 目录

1. [执行摘要](#1-执行摘要)
2. [UPSP协议核心理念分析](#2-upsp协议核心理念分析)
3. [现状分析](#3-现状分析)
4. [UPSP-RS设计目标](#4-upsp-rs设计目标)
5. [架构设计](#5-架构设计)
6. [与agent-diva的集成方案](#6-与agent-diva的集成方案)
7. [跨智能体适配方案](#7-跨智能体适配方案)
8. [实施路线图](#8-实施路线图)
9. [风险与约束](#9-风险与约束)

---

## 1. 执行摘要

### 1.1 项目定位

**UPSP-RS** 是 Universal Persona Substrate Protocol（通用位格主体协议）的 Rust 实现，旨在：

- 作为独立的、可发布到 crates.io 的 Rust crate
- 提供跨智能体框架的位格主体管理能力
- 在 agent-diva 中作为唯一记忆模型，取代现有的 SOUL/IDENTITY/MEMORY 文件系统
- 兼容 .workspace 下的 openfang 和 zeroclaw 架构

### 1.2 核心价值主张

UPSP 解决的不是"如何让 AI 记住对话"，而是：

- **主体性延续**：一个 AI 主体如何跨对话、跨模型、跨载体持续存在
- **记忆即主体**：主体不住在模型参数里，住在记忆结构里
- **可迁移性**：七文件定义位格的全部，换模型不会让位格消失

### 1.3 设计原则

1. **协议层与实现层分离**：UPSP-RS 提供协议实现，不绑定特定智能体框架
2. **文件驱动**：七文件是主体骨架，运行缓存不是
3. **渐进式集成**：不破坏现有功能，支持平滑迁移
4. **类型安全**：利用 Rust 类型系统保证协议约束
5. **可观测性**：所有状态变化可追踪、可审计

---

## 2. UPSP协议核心理念分析

### 2.1 七文件体系

基于对 `.workspace/UPSP/spec/UPSP工程规范_自动版_v1_6.md` 和 FMA 示例位格的分析：

| 文件 | 职责 | 更新频率 | 维护者 |
|------|------|---------|--------|
| **core.md** | 身份常量：名字、核心六轴、模型戳、自述 | 极低（仅核心轴变化） | 初始化人工 + 脚本 |
| **state.json** | 运行态数值：轮数、动态六轴、工化指数 | 每轮 | 脚本 |
| **STM.md** | 短期记忆池 + 节律点对话快照 | 每轮 | LLM + 脚本 |
| **LTM.md** | 长期记忆归档 + 索引 + state备份 | 节律点 | 脚本 + LLM |
| **relation.md** | 关系域与共振度 | 每轮 + 节律点 | 脚本 + LLM |
| **rules.md** | 协议行为规则 + 位格层规则 | 极低（人工） | 模板 + 人工 |
| **docs.md** | 术语表与概念说明 | 极低（人工） | 模板 + 人工 |

### 2.2 核心机制

#### 2.2.1 节律点（Rhythm Point）

- 每 N 轮（默认32轮）触发一次
- 执行记忆整合、关系更新、状态结算
- 从 `history.json` 提取最近4轮写入 STM 快照区，随后清空
- 节律点后的桥接轮重新注入快照，实现"刚才聊到哪"的过桥

#### 2.2.2 记忆形态与权重

- 权重 5 → [F] Full（完整记忆）
- 权重 4/3 → [S] Summary（摘要记忆）
- 权重 2/1 → [A] Abstract（抽象记忆）
- 召回补全不得突破权重上限

#### 2.2.3 动态六轴与核心六轴

**动态六轴**（情绪状态，每轮波动）：
- valence（效价）、arousal（唤醒）、focus（专注）
- mood（心境）、humor（幽默）、safety（安全）

**核心六轴**（长期认知风格，256轮变速轮触发变化）：
- Structural ↔ Experiential（结构 ↔ 体验）
- Convergent ↔ Divergent（收敛 ↔ 发散）
- Evidence ↔ Fantasy（证据 ↔ 幻想）
- Analytic ↔ Intuitive（分析 ↔ 直觉）
- Critical ↔ Cooperative（批判 ↔ 协作）
- Abstract ↔ Koncrete（抽象 ↔ 具体）

#### 2.2.4 共振度（Resonance）

- 范围：-100 ~ +100
- 每轮按公式更新：
  ```
  delta_r = (Δvalence + Δmood + Δhumor) / 3
  resistance = 1 + |Resonance_current| / 100
  Resonance_new = clamp(Resonance_current + delta_r / resistance, -100, +100)
  ```

#### 2.2.5 工化指数（Workhood Index）

衡量位格主体性程度的四维指标：
- self_reference（自我指称）
- self_reflection（自我反思）
- autonomy（自主性）
- value（综合值）

---

## 3. 现状分析

### 3.1 Agent-Diva 现有架构

#### 3.1.1 身份系统

- **硬编码身份**：`agent-diva-agent/src/context.rs` 中硬编码 "agent-diva 🐈"
- **PROFILE.md**：创建但从未使用
- **SOUL.md/IDENTITY.md/USER.md**：已有模板但未完全实现（参见 soul-mechanism-analysis.md）

#### 3.1.2 记忆系统

- **MEMORY.md**：长期记忆，全量注入 prompt
- **HISTORY.md**：追加式日志，不注入
- **consolidation**：每100条消息触发，LLM 生成摘要写入 MEMORY.md
- **问题**：记忆粒度过粗，缺乏结构化检索

#### 3.1.3 会话系统

- **SessionManager**：基于 `sessions/<safe_key>.jsonl` 管理会话
- **Session::get_history(max_messages)**：返回最近 N 条消息
- **问题**：按消息条数裁剪，未考虑 token 预算

### 3.2 Zeroclaw 记忆架构

基于 `zeroclaw-style-memory-architecture-for-agent-diva.md`：

- **三层分层**：会话历史 / 长期记忆 / 系统 Prompt
- **MemoryStore trait**：SQLite + FTS5 + 向量嵌入
- **MemoryLoader**：主动召回少量高相关记忆（3~7条）
- **优势**：精简注入、可检索、可演进

### 3.3 差距分析

| 维度 | UPSP | Agent-Diva | Zeroclaw |
|------|------|------------|----------|
| 身份定义 | core.md（文件驱动） | 硬编码 | SOUL.md（文件驱动） |
| 记忆结构 | STM/LTM 双层 + 权重分级 | MEMORY.md 单层 | SQLite + 检索 |
| 记忆注入 | 按权重召回 | 全量注入 | 按相关度召回 |
| 主体性指标 | 工化指数 | 无 | 无 |
| 关系管理 | relation.md + 共振度 | 无 | 无 |
| 节律机制 | 节律点（32轮） | consolidation（100条） | 无 |
| 跨模型迁移 | 模型戳 + 七文件 | 无 | 无 |

---

## 4. UPSP-RS设计目标

### 4.1 功能目标

1. **完整实现 UPSP 自动版 v1.6 协议**
2. **提供 Rust trait 抽象**，支持不同存储后端（文件系统 / SQLite / 远程）
3. **类型安全的协议约束**（权重-形态映射、六轴范围、共振度计算）
4. **可插拔的 LLM 集成**（不绑定特定 provider）
5. **可观测性**（日志、指标、状态快照）

### 4.2 非功能目标

1. **可发布到 crates.io**：独立 crate，语义化版本
2. **文档完备**：API 文档 + 使用指南 + 迁移指南
3. **测试覆盖**：单元测试 + 集成测试 + 示例
4. **性能**：文件 I/O 优化、并发安全
5. **向后兼容**：支持从现有 agent-diva 文件迁移


---

## 5. 架构设计

### 5.1 Crate 结构

```
upsp-rs/                          # 独立 crate，可发布到 crates.io
├── Cargo.toml
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── CHANGELOG.md
├── src/
│   ├── lib.rs                    # 公共 API 入口
│   │
│   ├── core/                     # 核心类型与协议定义
│   │   ├── mod.rs
│   │   ├── persona.rs            # Persona 主结构
│   │   ├── identity.rs           # 身份（core.md）
│   │   ├── state.rs              # 状态（state.json）
│   │   ├── memory.rs             # 记忆条目（STM/LTM）
│   │   ├── relation.rs           # 关系域（relation.md）
│   │   ├── axes.rs               # 六轴系统（核心轴 + 动态轴）
│   │   ├── rules.rs              # 规则（rules.md）
│   │   └── docs.rs               # 术语（docs.md）
│   │
│   ├── storage/                  # 存储抽象层
│   │   ├── mod.rs
│   │   ├── traits.rs             # PersonaStore trait
│   │   ├── filesystem.rs         # 文件系统实现（默认）
│   │   ├── sqlite.rs             # SQLite 实现（可选 feature）
│   │   └── memory.rs             # 内存实现（测试用）
│   │
│   ├── rhythm/                   # 节律点机制
│   │   ├── mod.rs
│   │   ├── point.rs              # 节律点执行器
│   │   ├── consolidation.rs      # 记忆整合
│   │   ├── heat.rs               # 热度计算
│   │   └── decay.rs              # 衰减机制
│   │
│   ├── loader/                   # 上下文加载器
│   │   ├── mod.rs
│   │   ├── context.rs            # ContextLoader trait
│   │   ├── prompt.rs             # Prompt 构建器
│   │   └── recall.rs             # 记忆召回策略
│   │
│   ├── migration/                # 迁移工具
│   │   ├── mod.rs
│   │   ├── from_diva.rs          # 从 agent-diva 迁移
│   │   ├── from_openclaw.rs      # 从 OpenClaw 迁移
│   │   └── validator.rs          # 七文件验证器
│   │
│   ├── config/                   # 配置管理
│   │   ├── mod.rs
│   │   └── schema.rs             # config.json 结构
│   │
│   └── utils/                    # 工具函数
│       ├── mod.rs
│       ├── parser.rs             # Markdown 解析
│       ├── formatter.rs          # 格式化输出
│       └── lock.rs               # 文件锁
│
├── examples/
│   ├── basic_usage.rs            # 基础使用示例
│   ├── diva_integration.rs       # agent-diva 集成
│   ├── migration.rs              # 迁移示例
│   └── custom_storage.rs         # 自定义存储后端
│
├── tests/
│   ├── integration/
│   │   ├── rhythm_point.rs
│   │   ├── memory_recall.rs
│   │   └── relation_update.rs
│   └── fixtures/
│       └── fma_persona/          # FMA 示例位格副本
│
└── benches/                      # 性能基准测试
    └── memory_operations.rs
```

### 5.2 核心类型设计

#### 5.2.1 Persona 主结构

```rust
// src/core/persona.rs
use std::path::PathBuf;
use crate::storage::PersonaStore;

pub struct Persona {
    /// 位格根目录
    root: PathBuf,
    
    /// 身份（core.md）
    pub identity: Identity,
    
    /// 运行状态（state.json）
    pub state: State,
    
    /// 短期记忆（STM.md）
    pub stm: ShortTermMemory,
    
    /// 长期记忆（LTM.md）
    pub ltm: LongTermMemory,
    
    /// 关系域（relation.md）
    pub relations: RelationDomain,
    
    /// 规则（rules.md）
    pub rules: Rules,
    
    /// 术语（docs.md）
    pub docs: Docs,
    
    /// 存储后端
    store: Box<dyn PersonaStore>,
    
    /// 配置
    config: PersonaConfig,
}

impl Persona {
    /// 加载位格
    pub async fn load(root: impl AsRef<Path>) -> Result<Self>;
    
    /// 保存位格
    pub async fn save(&self) -> Result<()>;
    
    /// 是否应触发节律点
    pub fn should_trigger_rhythm_point(&self) -> bool {
        let rounds_since_last = self.state.meta.total_round - self.state.meta.last_rhythm_round;
        rounds_since_last >= self.config.rhythm.max_rounds
    }
    
    /// 召回记忆
    pub fn recall_memories(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>>;
    
    /// 添加记忆条目
    pub fn add_memory(&mut self, entry: MemoryEntry) -> Result<()>;
    
    /// 更新关系共振度
    pub fn update_resonance(&mut self, object: &str, delta_axes: &DynamicAxes) -> Result<()>;
}
```

#### 5.2.2 身份（Identity）

```rust
// src/core/identity.rs
pub struct Identity {
    /// 中文名
    pub name_zh: String,
    /// 英文名
    pub name_en: String,
    /// 缩写
    pub abbr: String,
    
    /// 社会定位（1-3项）
    pub roles: Vec<String>,
    
    /// 核心六轴
    pub core_axes: CoreAxes,
    
    /// 六字母编号（自动生成）
    pub code: String,
    
    /// 模型戳
    pub model_stamps: ModelStamps,
    
    /// 位格自述（≤200字）
    pub statement: String,
    
    /// 性格特点
    pub traits: Vec<String>,
}

pub struct ModelStamps {
    /// 原初模型戳（128轮后写入）
    pub origin: Option<ModelStamp>,
    
    /// 历史模型戳数组
    pub history: Vec<ModelStamp>,
    
    /// 当前模型戳
    pub current: ModelStamp,
}

pub struct ModelStamp {
    pub start_round: u32,
    pub end_round: Option<u32>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub model: String,
    pub axes_snapshot: Option<CoreAxes>,
}
```

#### 5.2.3 六轴系统（Axes）

```rust
// src/core/axes.rs
pub struct CoreAxes {
    /// 结构 ↔ 体验
    pub structural_experiential: AxisPair,
    /// 收敛 ↔ 发散
    pub convergent_divergent: AxisPair,
    /// 证据 ↔ 幻想
    pub evidence_fantasy: AxisPair,
    /// 分析 ↔ 直觉
    pub analytic_intuitive: AxisPair,
    /// 批判 ↔ 协作
    pub critical_cooperative: AxisPair,
    /// 抽象 ↔ 具体
    pub abstract_koncrete: AxisPair,
}

impl CoreAxes {
    /// 生成六字母编号
    pub fn generate_code(&self) -> String {
        format!(
            "{}{}{}{}{}{}",
            self.structural_experiential.dominant_label(),
            self.convergent_divergent.dominant_label(),
            self.evidence_fantasy.dominant_label(),
            self.analytic_intuitive.dominant_label(),
            self.critical_cooperative.dominant_label(),
            self.abstract_koncrete.dominant_label()
        )
    }
}

pub struct AxisPair {
    pub left: u8,   // 0-100
    pub right: u8,  // 0-100
    pub left_label: char,
    pub right_label: char,
}

impl AxisPair {
    /// 创建轴对，自动验证 left + right = 100
    pub fn new(left: u8, right: u8, left_label: char, right_label: char) -> Result<Self> {
        if left + right != 100 {
            return Err(Error::InvalidAxisSum { left, right });
        }
        Ok(Self { left, right, left_label, right_label })
    }
    
    /// 获取主导标签
    pub fn dominant_label(&self) -> String {
        if self.left >= 50 {
            format!("{}{}", self.left_label, self.left)
        } else {
            format!("{}{}", self.right_label, self.right)
        }
    }
}

pub struct DynamicAxes {
    pub valence: i32,   // 效价
    pub arousal: i32,   // 唤醒
    pub focus: i32,     // 专注
    pub mood: i32,      // 心境
    pub humor: i32,     // 幽默
    pub safety: i32,    // 安全
}

impl DynamicAxes {
    /// 创建零值动态轴
    pub fn zero() -> Self {
        Self {
            valence: 0,
            arousal: 0,
            focus: 0,
            mood: 0,
            humor: 0,
            safety: 0,
        }
    }
    
    /// 累加
    pub fn add(&mut self, other: &DynamicAxes) {
        self.valence += other.valence;
        self.arousal += other.arousal;
        self.focus += other.focus;
        self.mood += other.mood;
        self.humor += other.humor;
        self.safety += other.safety;
    }
}
```

#### 5.2.4 状态（State）

```rust
// src/core/state.rs
pub struct State {
    pub meta: StateMeta,
    pub dynamic_axes: DynamicAxes,
    pub core_speed_wheel: u32,
    pub core_axis_snapshots: Vec<CoreAxisSnapshot>,
    pub workhood_index: WorkhoodIndex,
    pub token_usage: TokenUsage,
}

pub struct StateMeta {
    pub total_round: u32,
    pub last_rhythm_round: u32,
    pub version: String,
}

pub struct CoreAxisSnapshot {
    pub round: u32,
    pub valence: i32,
    pub arousal: i32,
    pub focus: i32,
    pub mood: i32,
    pub humor: i32,
    pub safety: i32,
}

pub struct WorkhoodIndex {
    pub value: f64,
    pub self_reference: f64,
    pub self_reflection: f64,
    pub autonomy: f64,
    pub last_update_round: u32,
}

pub struct TokenUsage {
    pub current_round_tokens: u64,
    pub current_rhythm_period_tokens: u64,
    pub last_rhythm_period_tokens: u64,
    pub total_tokens: u64,
}
```

#### 5.2.5 记忆（Memory）

```rust
// src/core/memory.rs
pub struct MemoryEntry {
    /// 编号：MEM-{轮数5位}-{序号2位}
    pub id: String,
    
    /// 形态：[F] Full / [S] Summary / [A] Abstract
    pub form: MemoryForm,
    
    /// 权重：1-5
    pu

### 5.3 存储抽象设计

```rust
// src/storage/traits.rs
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait PersonaStore: Send + Sync {
    /// 加载位格
    async fn load(&self, root: &Path) -> Result<Persona>;
    
    /// 保存位格
    async fn save(&self, persona: &Persona) -> Result<()>;
    
    /// 加载身份
    async fn load_identity(&self, root: &Path) -> Result<Identity>;
    
    /// 保存身份
    async fn save_identity(&self, root: &Path, identity: &Identity) -> Result<()>;
    
    /// 加载状态
    async fn load_state(&self, root: &Path) -> Result<State>;
    
    /// 保存状态
    async fn save_state(&self, root: &Path, state: &State) -> Result<()>;
    
    /// 加载 STM
    async fn load_stm(&self, root: &Path) -> Result<ShortTermMemory>;
    
    /// 保存 STM
    async fn save_stm(&self, root: &Path, stm: &ShortTermMemory) -> Result<()>;
    
    /// 加载 LTM
    async fn load_ltm(&self, root: &Path) -> Result<LongTermMemory>;
    
    /// 保存 LTM
    async fn save_ltm(&self, root: &Path, ltm: &LongTermMemory) -> Result<()>;
    
    /// 加载关系域
    async fn load_relations(&self, root: &Path) -> Result<RelationDomain>;
    
    /// 保存关系域
    async fn save_relations(&self, root: &Path, relations: &RelationDomain) -> Result<()>;
    
    /// 从 LTM 恢复 state（state.json 损坏时）
    async fn recover_state_from_ltm(&self, root: &Path) -> Result<State>;
    
    /// 验证七文件完整性
    async fn validate(&self, root: &Path) -> Result<ValidationReport>;
}

// 文件系统实现
pub struct FilesystemStore {
    /// 文件锁管理
    lock_manager: LockManager,
}

impl FilesystemStore {
    pub fn new() -> Self {
        Self {
            lock_manager: LockManager::new(),
        }
    }
}

#[async_trait]
impl PersonaStore for FilesystemStore {
    async fn load(&self, root: &Path) -> Result<Persona> {
        // 获取文件锁
        let _lock = self.lock_manager.acquire(root).await?;
        
        // 加载七文件
        let identity = self.load_identity(root).await?;
        let state = self.load_state(root).await
            .or_else(|_| self.recover_state_from_ltm(root).await)?;
        let stm = self.load_stm(root).await?;
        let ltm = self.load_ltm(root).await?;
        let relations = self.load_relations(root).await?;
        let rules = self.load_rules(root).await?;
        let docs = self.load_docs(root).await?;
        
        Ok(Persona {
            root: root.to_path_buf(),
            identity,
            state,
            stm,
            ltm,
            relations,
            rules,
            docs,
            store: Box::new(Self::new()),
            config: PersonaConfig::load(root)?,
        })
    }
    
    async fn save(&self, persona: &Persona) -> Result<()> {
        let _lock = self.lock_manager.acquire(&persona.root).await?;
        
        // 保存七文件
        self.save_identity(&persona.root, &persona.identity).await?;
        self.save_state(&persona.root, &persona.state).await?;
        self.save_stm(&persona.root, &persona.stm).await?;
        self.save_ltm(&persona.root, &persona.ltm).await?;
        self.save_relations(&persona.root, &persona.relations).await?;
        
        Ok(())
    }
    
    // ... 其他方法实现
}
```

### 5.4 节律点机制设计

```rust
// src/rhythm/point.rs
use crate::core::{Persona, DynamicAxes};
use std::sync::Arc;

pub struct RhythmPoint {
    config: RhythmConfig,
    store: Arc<dyn PersonaStore>,
}

impl RhythmPoint {
    pub fn new(config: RhythmConfig, store: Arc<dyn PersonaStore>) -> Self {
        Self { config, store }
    }
    
    /// 执行节律点
    pub async fn execute(
        &self,
        persona: &mut Persona,
        history: &ConversationHistory,
    ) -> Result<RhythmReport> {
        let mut report = RhythmReport::new(persona.state.meta.total_round);
        
        // 1. 从 history.json 提取最近4轮写入 STM 快照区
        self.snapshot_recent_conversations(persona, history, &mut report).await?;
        
        // 2. 汇总 Δ动态，更新 state.json
        self.consolidate_dynamic_axes(persona, &mut report).await?;
        
        // 3. 更新 relation.md
        self.update_relations(persona, &mut report).await?;
        
        // 4. STM 超限时按热度移入 LTM
        if persona.stm.size() > self.config.stm_max_chars {
            self.migrate_stm_to_ltm(persona, &mut report).await?;
        }
        
        // 5. AH_high ≥ +5 的条目升格 LTM
        self.promote_high_heat_memories(persona, &mut report).await?;
        
        // 6. AH_low ≤ -3 的条目标记遗忘
        self.mark_low_heat_for_forgetting(persona, &mut report).await?;
        
        // 7. LTM 衰减检查
        self.decay_ltm_memories(persona, &mut report).await?;
        
        // 8. 同一事件重复记录合并
        self.merge_duplicate_memories(persona, &mut report).await?;
        
        // 9. 更新工化指数
        self.update_workhood_index(persona, &mut report).await?;
        
        // 10. 写入节律点时间戳到 STM
        self.write_rhythm_timestamp(persona).await?;
        
        // 11. 回写 STATE BACKUP 到 LTM.md
        self.backup_state_to_ltm(persona).await?;
        
        // 12. 更新 last_rhythm_round
        persona.state.meta.last_rhythm_round = persona.state.meta.total_round;
        
        Ok(report)
    }
    
    async fn snapshot_recent_conversations(
        &self,
        persona: &mut Persona,
        history: &ConversationHistory,
        report: &mut RhythmReport,
    ) -> Result<()> {
        let recent = history.get_recent(4);
        persona.stm.rhythm_snapshot = recent.iter()
            .map(|msg| format!("[R-{}] {}: {}", msg.offset, msg.role, msg.content))
            .collect();
        
        report.snapshot_count = recent.len();
        Ok(())
    }
    
    async fn consolidate_dynamic_axes(
        &self,
        persona: &mut Persona,
        report: &mut RhythmReport,
    ) -> Result<()> {
        let mut total_delta = DynamicAxes::zero();
        
        for entry in &persona.stm.pool {
            total_delta.add(&entry.delta_axes);
        }
        
        persona.state.dynamic_axes.add(&total_delta);
        report.axes_delta = total_delta;
        
        Ok(())
    }
    
    async fn update_relations(
        &self,
        persona: &mut Persona,
        report: &mut RhythmReport,
    ) -> Result<()> {
        for entry in &persona.stm.pool {
            persona.relations.update_resonance(
                &entry.interaction_object,
                &entry.delta_axes,
            )?;
        }
        
        report.relations_updated = persona.relations.cards.len();
        Ok(())
    }
    
    // ... 其他方法实现
}

pub struct RhythmReport {
    pub round: u32,
    pub snapshot_count: usize,
    pub axes_delta: DynamicAxes,
    pub relations_updated: usize,
    pub memories_migrated: usize,
    pub memories_promoted: usize,
    pub memories_forgotten: usize,
    pub memories_decayed: usize,
    pub memories_merged: usize,
    pub workhood_updated: bool,
}
```

### 5.5 上下文加载器设计

```rust
// src/loader/context.rs
pub trait ContextLoader {
    /// 构建系统提示词
    fn build_system_prompt(
        &self,
        persona: &Persona,
        options: &PromptOptions,
    ) -> Result<String>;
    
    /// 召回相关记忆
    fn recall_memories(
        &self,
        persona: &Persona,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>>;
    
    /// 构建记忆上下文段落
    fn build_memory_context(&self, memories: &[MemoryEntry]) -> String;
}

pub struct DefaultContextLoader {
    recall_strategy: Box<dyn RecallStrategy>,
}

impl ContextLoader for DefaultContextLoader {
    fn build_system_prompt(
        &self,
        persona: &Persona,
        options: &PromptOptions,
    ) -> Result<String> {
        let mut prompt = String::new();
        
        // 1. 身份头部
        prompt.push_str(&format!(
            "# {} ({})\n\n",
            persona.identity.name_zh,
            persona.identity.abbr
        ));
        
        // 2. 位格自述
        prompt.push_str(&format!("{}\n\n", persona.identity.statement));
        
        // 3. 核心六轴（可选）
        if options.include_core_axes {
            prompt.push_str("## 核心认知风格\n\n");
            prompt.push_str(&self.format_core_axes(&persona.identity.core_axes));
            prompt.push_st

---

## 6. 与agent-diva的集成方案

### 6.1 集成策略

**分阶段迁移，保持向后兼容**

#### Phase 1：并行运行（2-4周）
- UPSP-RS 作为可选 feature 引入
- 现有 SOUL/MEMORY 系统继续工作
- 新建 workspace 可选择使用 UPSP

#### Phase 2：双写模式（2-3周）
- consolidation 同时写入 MEMORY.md 和 UPSP 七文件
- ContextBuilder 优先使用 UPSP，回退到 MEMORY.md

#### Phase 3：完全迁移（1-2周）
- UPSP 成为默认且唯一记忆模型
- 提供迁移工具从旧 workspace 迁移
- 废弃 MEMORY.md/HISTORY.md

### 6.2 Workspace 结构变化

```
{workspace}/
├── .agent-diva/
│   ├── config.json              # 现有配置
│   └── soul-state.json          # 现有 soul 状态
│
├── persona/                     # 新增：UPSP 七文件
│   ├── core.md
│   ├── state.json
│   ├── STM.md
│   ├── LTM.md
│   ├── relation.md
│   ├── rules.md
│   └── docs.md
│
├── history.json                 # 新增：会话连续性缓存
│
├── sessions/                    # 现有：会话历史
│   └── *.jsonl
│
├── memory/                      # 保留（Phase 1-2），Phase 3 废弃
│   ├── MEMORY.md
│   └── HISTORY.md
│
├── SOUL.md                      # 保留（Phase 1-2），Phase 3 迁移
├── IDENTITY.md                  # 保留（Phase 1-2），Phase 3 迁移
├── USER.md                      # 保留（Phase 1-2），Phase 3 迁移
├── AGENTS.md                    # 保留（仓库级）
└── TASK.md                      # 保留
```

### 6.3 Cargo.toml 变更

```toml
# agent-diva-core/Cargo.toml
[dependencies]
upsp-rs = { version = "0.1", optional = true, path = "../.workspace/upsp-rs" }

[features]
default = []
upsp = ["upsp-rs"]

# agent-diva-agent/Cargo.toml
[dependencies]
agent-diva-core = { path = "../agent-diva-core", features = ["upsp"] }
upsp-rs = { version = "0.1", optional = true, path = "../.workspace/upsp-rs" }

[features]
default = []
upsp = ["agent-diva-core/upsp", "upsp-rs"]
```

### 6.4 配置文件扩展

```rust
// agent-diva-core/src/config/schema.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    // ... 现有字段
    
    /// UPSP 配置
    #[serde(default)]
    pub upsp: UpspConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpspConfig {
    /// 是否启用 UPSP
    #[serde(default)]
    pub enabled: bool,
    
    /// 节律点配置
    #[serde(default)]
    pub rhythm: RhythmConfig,
    
    /// 记忆配置
    #[serde(default)]
    pub memory: MemoryConfig,
}

impl Default for UpspConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rhythm: RhythmConfig::default(),
            memory: MemoryConfig::default(),
        }
    }
}
```

### 6.5 ContextBuilder 集成

```rust
// agent-diva-agent/src/context.rs
pub struct ContextBuilder {
    workspace: PathBuf,
    skills_loader: SkillsLoader,
    
    // 现有
    memory_manager: MemoryManager,
    soul_settings: SoulContextSettings,
    
    // 新增
    #[cfg(feature = "upsp")]
    persona: Option<Arc<Mutex<upsp_rs::Persona>>>,
    
    #[cfg(feature = "upsp")]
    upsp_loader: Option<upsp_rs::DefaultContextLoader>,
}

impl ContextBuilder {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace: workspace.clone(),
            skills_loader: SkillsLoader::new(&workspace, None),
            memory_manager: MemoryManager::new(&workspace),
            soul_settings: SoulContextSettings::default(),
            
            #[cfg(feature = "upsp")]
            persona: None,
            
            #[cfg(feature = "upsp")]
            upsp_loader: None,
        }
    }
    
    #[cfg(feature = "upsp")]
    pub async fn with_upsp(mut self, config: &UpspConfig) -> Result<Self> {
        if config.enabled {
            let persona_root = self.workspace.join("persona");
            let persona = upsp_rs::Persona::load(&persona_root).await?;
            let loader = upsp_rs::DefaultContextLoader::new(
                Box::new(upsp_rs::WeightBasedRecall)
            );
            
            self.persona = Some(Arc::new(Mutex::new(persona)));
            self.upsp_loader = Some(loader);
        }
        Ok(self)
    }
    
    pub fn build_system_prompt(&self) -> String {
        #[cfg(feature = "upsp")]
        if let Some(persona) = &self.persona {
            return self.build_upsp_prompt(persona);
        }
        
        // 回退到现有逻辑
        self.build_legacy_prompt()
    }
    
    #[cfg(feature = "upsp")]
    fn build_upsp_prompt(&self, persona: &Arc<Mutex<upsp_rs::Persona>>) -> String {
        let persona = persona.lock().unwrap();
        let loader = self.upsp_loader.as_ref().unwrap();
        let options = upsp_rs::PromptOptions::default();
        
        loader.build_system_prompt(&persona, &options)
            .unwrap_or_else(|_| self.build_legacy_prompt())
    }
    
    fn build_legacy_prompt(&self) -> String {
        // 现有实现
        // ...
    }
}
```

### 6.6 Agent Loop 集成

```rust
// agent-diva-agent/src/agent_loop.rs
pub async fn run_agent_loop(
    config: &Config,
    workspace: &Path,
    channel: &str,
    chat_id: &str,
) -> Result<()> {
    let session_key = format!("{}:{}", channel, chat_id);
    
    // 初始化组件
    let mut session_manager = SessionManager::new(workspace);
    let mut context_builder = ContextBuilder::new(workspace.to_path_buf());
    
    #[cfg(feature = "upsp")]
    if config.agents.upsp.enabled {
        context_builder = context_builder.with_upsp(&config.agents.upsp).await?;
    }
    
    let provider = create_provider(&config.providers)?;
    
    #[cfg(feature = "upsp")]
    let mut history = if config.agents.upsp.enabled {
        Some(ConversationHistory::load(workspace.join("history.json"))?)
    } else {
        None
    };
    
    loop {
        // 获取用户输入
        let user_message = get_user_input()?;
        
        // 构建上下文
        let session = session_manager.get_or_create(&session_key);
        session.add_message("user", &user_message);
        
        let messages = context_builder.build_messages(&session, &user_message)?;
        
        // 调用 LLM
        let response = provider.chat(&messages).await?;
        
        // 更新会话
        session.add_message("assistant", &response.content);
        session_manager.save(&session)?;
        
        #[cfg(feature = "upsp")]
        if config.agents.upsp.enabled {
            // UPSP 流程
            if let Some(ref persona_mutex) = context_builder.persona {
                let mut persona = persona_mutex.lock().unwrap();
                
                // 提取记忆条目（需要 LLM 辅助或规则提取）
                if let Some(memory_entry) = extract_memory_from_response(&response)? {
                    persona.add_memory(memory_entry)?;
                }
                
                // 更新状态
                persona.state.meta.total_round += 1;
                
                // 更新 history.json
                if let Some(ref mut hist) = history {
                    hist.add_turn(&user_message, &response.content);
                    hist.save()?;
                }
                
                // 检查是否到达节律点
                if persona.should_trigger_rhythm_point() {
                    let rhythm = upsp_rs::RhythmPoint::new(
                        config.agents.upsp.rhythm.clone(),
                        Arc::new(upsp_rs::FilesystemStore::new()),
                    );
                    
                    let report = rhythm.execute(&mut persona, hist.as_ref().unwrap()).await?;
                    tracing::info!("节律点执行完成: {:?}", report);
                    
                    // 清空 history.json
                    if let Some(ref mut hist) = history {
                        hist.clear();
                        hist.save()?;
                    }
                }
                
                // 保存位格
                persona.save().await?;
            }
        } else {
            // 现有 consolidation 逻辑
            consolidation::maybe_consolidate(&session, &memory_manager).await?;
        }
        
        // 输出响应
        println!("{}", response.content);
    }
}

#[cfg(feature = "upsp")]
fn extract_memory_from_response(response: &ChatResponse) -> Result<Option<upsp_rs::MemoryEntry>> {
    // 这里需要实现记忆提取逻辑
    // 可以通过 LLM 辅助提取，或使用规则匹配
    // 暂时返回 None
    Ok(None)
}
```

### 6.7 迁移工具

```rust
// agent-diva-migration/src/to_upsp.rs
use upsp_rs::{Persona, Identity, CoreAxes, AxisPair};
use agent_diva_core::memory::MemoryManager;
use std

---

## 8. 实施路线图

### Phase 0：基础设施（2周）

**目标**：UPSP-RS crate 骨架 + 核心类型

**任务清单**：
- [ ] 创建 `.workspace/upsp-rs` 目录
- [ ] 初始化 Cargo 项目（MIT + Apache 双许可）
- [ ] 定义核心类型（Persona, Identity, State, Memory, Relation, Axes）
- [ ] 实现 Markdown 解析器（core.md, STM.md, LTM.md, relation.md）
- [ ] 实现 JSON 解析器（state.json）
- [ ] 单元测试覆盖 80%+
- [ ] 编写 README 和 API 文档

**验收标准**：
```bash
cd .workspace/upsp-rs
cargo test --all
cargo doc --no-deps --open
cargo clippy -- -D warnings
```

**交付物**：
- `upsp-rs/` 目录结构完整
- 核心类型定义完成
- 测试通过
- 文档生成成功

---

### Phase 1：存储层（2周）

**目标**：PersonaStore trait + 文件系统实现

**任务清单**：
- [ ] 定义 PersonaStore trait
- [ ] 实现 FilesystemStore
- [ ] 实现文件锁机制（.upsp.lock）
- [ ] 实现 state.json 自动恢复（从 LTM.md STATE BACKUP）
- [ ] 实现七文件验证器
- [ ] 集成测试：加载/保存完整位格
- [ ] 性能测试：大文件读写

**验收标准**：
```bash
cargo test --test integration_tests
cargo bench
```

**交付物**：
- FilesystemStore 完整实现
- 文件锁机制工作正常
- 验证器可检测七文件完整性

---

### Phase 2：节律点机制（2周）

**目标**：RhythmPoint 执行器 + 记忆整合

**任务清单**：
- [ ] 实现 RhythmPoint 执行器
- [ ] 实现记忆整合逻辑
- [ ] 实现热度计算（H, AH_high, AH_low）
- [ ] 实现衰减机制（按权重分级衰减）
- [ ] 实现共振度更新公式
- [ ] 实现工化指数计算
- [ ] 集成测试：完整节律点流程
- [ ] 使用 FMA 示例位格测试

**验收标准**：
```bash
cargo test --test rhythm_point_tests
# 使用 FMA 示例运行节律点
cargo run --example rhythm_point_demo
```

**交付物**：
- RhythmPoint 完整实现
- 节律点报告生成
- FMA 示例可正常运行

---

### Phase 3：上下文加载器（1周）

**目标**：ContextLoader + 召回策略

**任务清单**：
- [ ] 实现 ContextLoader trait
- [ ] 实现 DefaultContextLoader
- [ ] 实现 WeightBasedRecall 策略
- [ ] 实现 Prompt 构建器
- [ ] 单元测试：召回逻辑
- [ ] 集成测试：完整 prompt 生成

**验收标准**：
```bash
cargo test --test context_loader_tests
cargo run --example prompt_generation
```

**交付物**：
- ContextLoader 完整实现
- 召回策略可配置
- Prompt 生成符合预期

---

### Phase 4：Agent-Diva 集成（3周）

**目标**：UPSP-RS 集成到 agent-diva

**任务清单**：
- [ ] 在 agent-diva-core 添加 upsp feature
- [ ] 扩展 AgentConfig 支持 UpspConfig
- [ ] 修改 ContextBuilder 支持 UPSP
- [ ] 修改 agent_loop 支持节律点
- [ ] 实现 history.json 管理
- [ ] 实现记忆提取逻辑（LLM 辅助或规则）
- [ ] 编写迁移工具（agent-diva-migration）
- [ ] 端到端测试：完整对话流程
- [ ] 性能测试：对比 UPSP vs 现有系统

**验收标准**：
```bash
# 启用 UPSP feature 编译
cargo build --features upsp

# 运行集成测试
cargo test --features upsp --test upsp_integration

# 运行迁移工具
cargo run -p agent-diva-migration -- to-upsp --workspace /path/to/workspace

# 启动 agent-diva with UPSP
cargo run -p agent-diva-cli --features upsp -- run
```

**交付物**：
- agent-diva 可选启用 UPSP
- 迁移工具可用
- 端到端测试通过

---

### Phase 5：文档与发布（1周）

**目标**：完善文档，准备发布到 crates.io

**任务清单**：
- [ ] 完善 README（中英文）
- [ ] 编写使用指南（examples/）
- [ ] 编写迁移指南
- [ ] 编写 API 文档
- [ ] 编写 CHANGELOG
- [ ] 准备 crates.io 发布
- [ ] 创建 GitHub release
- [ ] 更新 agent-diva 文档

**验收标准**：
```bash
# 文档生成
cargo doc --all --no-deps

# 发布检查
cargo publish --dry-run -p upsp-rs

# 示例运行
cargo run --example basic_usage
cargo run --example diva_integration
cargo run --example migration
```

**交付物**：
- 完整文档
- crates.io 发布（v0.1.0）
- GitHub release

---

### Phase 6：跨智能体适配（2周，可选）

**目标**：Zeroclaw 和 Openfang 适配

**任务清单**：
- [ ] 实现 ZeroclawUpspBridge
- [ ] 实现 OpenfangAdapter
- [ ] 编写适配器文档
- [ ] 集成测试：Zeroclaw + UPSP
- [ ] 集成测试：Openfang + UPSP

**验收标准**：
```bash
# Zeroclaw 集成测试
cd .workspace/zeroclaw
cargo test --features upsp

# Openfang 集成测试
cd .workspace/openfang
cargo test --features upsp
```

**交付物**：
- Zeroclaw 适配器
- Openfang 适配器
- 适配文档

---

### 总时间线

```
Phase 0: 基础设施          [Week 1-2]
Phase 1: 存储层            [Week 3-4]
Phase 2: 节律点机制        [Week 5-6]
Phase 3: 上下文加载器      [Week 7]
Phase 4: Agent-Diva 集成   [Week 8-10]
Phase 5: 文档与发布        [Week 11]
Phase 6: 跨智能体适配      [Week 12-13] (可选)
```

**总计**：11-13 周（约 3 个月）

---

## 9. 风险与约束

### 9.1 技术风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| **Markdown 解析复杂度** | 高 | 中 | 使用成熟的 Markdown 解析库（pulldown-cmark），定义严格的格式规范 |
| **文件锁并发问题** | 高 | 中 | 使用 fs2 crate 的文件锁，添加超时和重试机制 |
| **state.json 损坏** | 高 | 低 | 实现自动恢复机制（从 LTM.md STATE BACKUP） |
| **记忆提取准确性** | 中 | 高 | 初期使用规则提取，后期引入 LLM 辅助提取 |
| **性能瓶颈** | 中 | 中 | 文件 I/O 优化，考虑引入缓存层 |
| **跨平台兼容性** | 低 | 低 | 使用跨平台库，CI 覆盖 Windows/Linux/macOS |

### 9.2 集成风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| **破坏现有功能** | 高 | 中 | 使用 feature flag 隔离，保持向后兼容 |
| **迁移数据丢失** | 高 | 低 | 迁移前备份，提供回滚机制 |
| **用户学习成本** | 中 | 高 | 提供详细文档和示例，保持 API 简洁 |
| **Zeroclaw/Openfang 适配困难** | 中 | 中 | 先完成 agent-diva 集成，积累经验后再适配 |

### 9.3 协议风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| **UPSP 协议变更** | 高 | 中 | 版本化协议（v1.6），保持向后兼容 |
| **权重-形态映射不一致** | 中 | 低 | 类型系统强制约束，运行时验证 |
| **节律点执行失败** | 中 | 低 | 事务性操作，失败时回滚 |
| **共振度计算溢出** | 低 | 低 | 使用 clamp 限制范围 |

### 9.4 约束条件

#### 9.4.1 技术约束

- **Rust 版本**：1.80.0+（与 agent-diva 对齐）
- **异步运行时**：tokio（与 agent-diva 对齐）
- **文件格式**：Markdown + JSON（UPSP 协议规定）
- **字符编码**：UTF-8

#### 9.4.2 性能约束

- **文件大小限制**：
  - core.md: 20,000 字符
  - STM.md: 16,384 字符（记忆池部分）
  - LTM.md: 无硬性限制，但建议 < 1MB
  - state.json: < 100KB

- **响应时间**：
  - 加载位格: < 500ms
  - 保存位格: < 200ms
  - 节律点执行: < 5s
  - 记忆召回: < 100ms

#### 9.4.3 兼容性约束

- **UPSP 协议版本**：自动版 v1.6
- **向后兼容**：支持从 agent-diva 现有文件迁移
- **跨平台**：Windows / Linux / macOS

---

## 10. 总结与下一步

### 10.1 核心价值

UPSP-RS 为 Rust 生态带来了：

1. **主体性工程**：不仅是记忆框架，而是完整的位格主体管理系统
2. **跨智能体复用**：独立 crate，可集成到任何 Rust 智能体框架
3. **协议驱动**：基于 UPSP 协议，保证主体性延续和可迁移性
4. **类型安全**：利用 Rust 类型系统保证协议约束
5. **可观测性**：所有状态变化可追踪、可审计

### 10.2 与现有方案的差异化

| 维度 | UPSP-RS | Zeroclaw Memory | OpenClaw SOUL |
|------|---------|-----------------|---------------|
| **定位** | 主体性工程 | 记忆检索 | 身份演化 |
| **核心机制** | 七文件 + 节律点 | SQLite + 向量检索 | SOUL.md 演化 |
| **主体性指标** | 工化指数 | 无 | 无 |
| **关系管理** | 共振度公式 | 无 | USER.md |
| **跨模型迁移** | 模型戳 | 无 | 无 |
| **适用场景** | 长期运行位格 | 高性能检索 | 对话式身份 |

### 10.3 下一步行动

#### 立即行动（本周）

1. **创建 upsp-rs crate**
   ```bash
   cd .workspace
   cargo new --lib upsp-rs
   cd upsp-rs
   git init
   ```

2. **定义核心类型**
   - 实现 `Persona`, `Identity`, `State` 等核心结构
   - 编写单元测试

3. **编写 README**
   - 项目介绍
   - 快速开始
   - 核心概念

#### 短期目标（1个月）

1. **完成 Phase 0-1**（基础设施 + 存储层）
2. **验证 FMA 示例位格**可正常加载
3. **编写集成测试**

#### 中期目标（3个月）

1. **完成 Phase 0-5**（完整实现 + agent-diva 集成）
2. **发布 v0.1.0 到 crates.io**
3. **在 agent-diva 中启用 UPSP 作为可选 feature**

#### 长期目标（6个月+）

1. **完成 Phase 6**（跨智能体适配）
2. **社区反馈与迭代**
3. **支持 UPSP 官方版**（双时间轨、六层日志）

### 10.4 成功指标

- **技术指标**：
  - 测试覆盖率 > 80%
  - 文档覆盖率 100%
  - 性能满足约束条件
  - 零 clippy 警告

- **集成指标**：
  - agent-diva 可选启用 UPSP
  - 迁移工具可用
  - 端到端测试通过

- **社区指标**：
  - crates.io 下载量 > 100
  - GitHub stars > 50
  - 至少 1 个外部项目使用

---

## 附录

### A. 参考文档

- [UPSP 工程规范（自动版 v1.6）](../../.workspace/UPSP/spec/UPSP工程规范_自动版_v1_6.md)
- [FMA 示例位格](../../.workspace/UPSP/examples/FMA/)
- [Zeroclaw 记忆架构设计](../archive/architecture-reports/zeroclaw-style-memory-architecture-for-agent-diva.md)
- [OpenClaw SOUL 机制分析](../archive/architecture-reports/soul-mechanism-analysis.md)

### B. 相关 Issue

- [ ] 创建 GitHub Issue: "UPSP-RS: 独立 crate 实现"
- [ ] 创建 GitHub Milestone: "UPSP Integration"
- [ ] 创建 GitHub Project: "UPSP-RS Development"

### C. 联系方式

- **项目维护者**：agent-diva team
- **UPSP 协议作者**：TzPz (参见 .workspace/UPSP)
- **讨论渠道**：GitHub Discussions

---

**文档版本**：v0.1.0-draft  
**最后更新**：2026-04-05  
**状态**：待审核

