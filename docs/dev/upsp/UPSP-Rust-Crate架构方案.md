# UPSP 独立 Rust Crate 架构设计方案

**版本**：v0.1  
**日期**：2026年4月3日  
**目标**：将UPSP协议实现为独立Rust crate，可独立测试、发布、版本控制  

---

## 1. 设计原则

1. **独立可测试**：每个crate可单独测试、发布、版本控制
2. **松耦合**：UPSP引擎不依赖agent-diva-memory的实现细节
3. **可重用**：upsp-core/upsp-engine可供其他项目使用
4. **渐进式**：可以只部署upsp-core（数据类型），不启动引擎
5. **向后兼容**：现有agent-diva-memory功能不受影响

---

## 2. 总体架构

```
agent-diva-workspace/
│
├── agent-diva-upsp-core/         ← 第1层：协议基础（无外部依赖）
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── types/                # 七文件数据类型
│       ├── loader/                # 文件加载器
│       ├── serializer/            # 序列化/反序列化
│       └── validator/             # 验证器
│
├── agent-diva-upsp-engine/       ← 第2层：运行时引擎
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── engine.rs              # 主引擎
│       ├── state_machine.rs       # 状态转移
│       ├── scheduler.rs           # 节律与睡眠
│       ├── metrics.rs             # 六轴计算
│       └── memory_lifecycle.rs    # STM→LTM生命周期
│
├── agent-diva-memory/            ← 第3层：集成适配（现有改造）
│   └── src/
│       ├── upsp_compat/          # 与UPSP引擎的桥接
│       └── ...（现有代码）
│
└── agent-diva-core/              ← 基础层（保持不变）
```

---

## 3. Crate 1: agent-diva-upsp-core

### 3.1 Cargo.toml

```toml
[package]
name = "agent-diva-upsp-core"
version = "0.1.0"
edition = "2021"
rust-version = "1.80.0"
authors = ["mastwet@UndefineFoundation"]
license = "MIT"
description = "UPSP Protocol - Core types and data structures"
repository = "https://github.com/ProjectViVy/agent-diva"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4"] }

[dev-dependencies]
tempfile = "3.10"
```

### 3.2 模块结构

```
agent-diva-upsp-core/src/
├── lib.rs
├── core.rs           # CoreAxes(SCVARB六轴)
├── state.rs          # DynamicAxes, StateJson, FatigueState
├── memory.rs         # StmEntry, LtmRecord, LtmTier
├── relation.rs       # RelationVector
├── diary.rs          # DiaryEntry
├── rules.rs          # Rules.md 结构
├── config.rs         # Config.json 结构
└── validation.rs     # 规范校验
```

### 3.3 核心类型定义

#### core.rs - 核心六轴

```rust
use serde::{Deserialize, Serialize};

/// 核心六轴 (SCVARB)
/// 值范围：-100 ~ +100
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreAxes {
    /// S: Structure ↔ Experience
    pub structure_experience: i16,
    
    /// C: Convergence ↔ Divergence
    pub convergence_divergence: i16,
    
    /// V: Evidence ↔ Fantasy
    pub evidence_fantasy: i16,
    
    /// A: Analysis ↔ Intuition
    pub analysis_intuition: i16,
    
    /// R: Critique ↔ Collaboration
    pub critique_collaboration: i16,
    
    /// B: Abstract ↔ Concrete
    pub abstract_concrete: i16,
}

impl CoreAxes {
    pub fn new() -> Self {
        Self {
            structure_experience: 50,
            convergence_divergence: 50,
            evidence_fantasy: 50,
            analysis_intuition: 50,
            critique_collaboration: 50,
            abstract_concrete: 50,
        }
    }
    
    /// 获取位格编码 (如 "S50/C70/V60/A75/R55/B80")
    pub fn persona_code(&self) -> String {
        format!(
            "S{}/C{}/V{}/A{}/R{}/B{}",
            Self::encode_axis(self.structure_experience),
            Self::encode_axis(self.convergence_divergence),
            Self::encode_axis(self.evidence_fantasy),
            Self::encode_axis(self.analysis_intuition),
            Self::encode_axis(self.critique_collaboration),
            Self::encode_axis(self.abstract_concrete),
        )
    }
    
    fn encode_axis(value: i16) -> String {
        if value == 0 {
            "X".to_string()
        } else {
            value.abs().to_string()
        }
    }
}

/// 核心变轮数组
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreMutationRounds([u8; 6]);

impl CoreMutationRounds {
    pub const MAX: u8 = 8;
    
    pub fn new() -> Self {
        Self([1, 1, 1, 1, 1, 1])
    }
    
    pub fn get(&self, axis: usize) -> u8 {
        self.0.get(axis).copied().unwrap_or(1)
    }
    
    pub fn set(&mut self, axis: usize, value: u8) {
        if axis < 6 {
            self.0[axis] = value.min(Self::MAX);
        }
    }
}

/// 模型戳
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStamp {
    pub original: String,
    pub history: Vec<ModelStampEntry>,
    pub current: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStampEntry {
    pub stage: usize,
    pub start: String,
    pub end: Option<String>,
    pub rounds: usize,
    pub axes_snapshot: String,
}
```

#### state.rs - 动态状态

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 动态六轴
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicAxes {
    pub valence: AxisState,
    pub arousal: AxisState,
    pub focus: AxisState,
    pub mood: AxisState,
    pub humor: AxisState,
    pub safety: AxisState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisState {
    pub value: i16,           // -100 ~ +100
    pub zone: u8,             // 1-20区间
    pub drift: u8,            // 每轮+1，上限8
    pub last_change_round: usize,
    pub last_reason: String,
    pub last_stm_ids: Vec<String>,
}

impl AxisState {
    pub fn new(initial: i16) -> Self {
        Self {
            value: initial,
            zone: Self::calculate_zone(initial),
            drift: 3,
            last_change_round: 0,
            last_reason: String::new(),
            last_stm_ids: Vec::new(),
        }
    }
    
    /// 计算20区间
    pub fn calculate_zone(value: i16) -> u8 {
        match value {
            v if v < -90 => 1,
            v if v < -80 => 2,
            v if v < -70 => 3,
            v if v < -60 => 4,
            v if v < -50 => 5,
            v if v < -40 => 6,
            v if v < -30 => 7,
            v if v < -20 => 8,
            v if v < -10 => 9,
            v if v < 0 => 10,
            v if v < 10 => 11,
            v if v < 20 => 12,
            v if v < 30 => 13,
            v if v < 40 => 14,
            v if v < 50 => 15,
            v if v < 60 => 16,
            v if v < 70 => 17,
            v if v < 80 => 18,
            v if v < 90 => 19,
            _ => 20,
        }
    }
}

/// 疲劳状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatigueState {
    /// 距上次睡眠的小时数
    pub time_since_sleep_hours: f64,
    /// 距上次日志的字符积累
    pub log_chars_since_last_log: usize,
}

/// State.json 主结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateJson {
    pub meta: StateMeta,
    pub core_speed_wheel: u8,
    pub modes: ModesConfig,
    pub dynamic_axes: DynamicAxes,
    pub workhood_index: WorkhoodIndex,
    pub fatigue: FatigueState,
    pub last_sleep_start: Option<DateTime<Utc>>,
    pub last_sleep_end: Option<DateTime<Utc>>,
    pub last_log_time: DateTime<Utc>,
    pub extensions: ExtensionsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMeta {
    pub total_round: usize,
    pub daily_round: usize,
    pub last_update: DateTime<Utc>,
    pub current_time: DateTime<Utc>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModesConfig {
    pub work_mode: String,
    pub thinking_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkhoodIndex {
    pub value: f32,
    pub self_reference: u8,
    pub self_reflection: u8,
    pub autonomy: u8,
    pub last_update_round: usize,
    pub last_update_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionsConfig {
    pub dreams: bool,
}
```

#### memory.rs - 记忆结构

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 记忆类型标记
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    /// [F] 完整记忆 - 原始写入，哪怕只有10字
    Full,
    /// [S] 摘要记忆 - 由[F]压缩而来
    Summary,
    /// [A] 梗概记忆 - 由[S]压缩而来
    Abstract,
}

/// LTM 层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LtmTier {
    /// Active - 来源STM升格
    Active,
    /// Forgotten - 来源STM遗忘
    Forgotten,
    /// Archive - 来源Forgotten降级
    Archive,
    /// Pinned - 永久锁定
    Pinned,
    /// Skills - 调用≥16次的技能
    Skills,
}

/// STM 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StmEntry {
    pub id: String,           // MEM-YYYYMMDD-XXXX-XXXX
    pub memory_type: MemoryType,
    pub timestamp: DateTime<Utc>,
    pub entry_round: usize,
    pub daily_round: usize,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub heat: f32,            // 热度 H
    pub ah_high: i8,          // 升格计数器
    pub ah_low: i8,           // 遗忘计数器
    pub zone: String,
    pub ltm_status: String,
    pub locked: bool,
    pub dynamic_influence: DynamicInfluence,
    pub relation_influence: RelationInfluence,
    pub workhood_influence: WorkhoodInfluence,
}

/// LTM 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtmRecord {
    pub id: String,
    pub memory_type: MemoryType,
    pub tier: LtmTier,
    pub timestamp: DateTime<Utc>,
    pub entry_round: usize,
    pub last_called_round: Option<usize>,
    pub last_called_time: Option<DateTime<Utc>>,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub heat: f32,
    pub call_count: usize,
}

/// 动态影响
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DynamicInfluence {
    pub valence: i16,
    pub arousal: i16,
    pub focus: i16,
    pub mood: i16,
    pub humor: i16,
    pub safety: i16,
}
```

---

## 4. Crate 2: agent-diva-upsp-engine

### 4.1 Cargo.toml

```toml
[package]
name = "agent-diva-upsp-engine"
version = "0.1.0"
edition = "2021"
rust-version = "1.80.0"
authors = ["mastwet@UndefineFoundation"]
license = "MIT"
description = "UPSP Protocol - Runtime execution engine"

[dependencies]
agent-diva-upsp-core = { path = "../agent-diva-upsp-core" }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4"] }
tracing = "0.1"
thiserror = "1.0"

tokio = { version = "1.35", optional = true }
tokio-cron-scheduler = { version = "0.10", optional = true }

[features]
default = []
async-runtime = ["tokio", "tokio-cron-scheduler"]

[dev-dependencies]
tempfile = "3.10"
```

### 4.2 模块结构

```
agent-diva-upsp-engine/src/
├── lib.rs
├── engine.rs          # UpspEngine 主引擎
├── state_machine.rs   # 状态转移逻辑
├── scheduler.rs       # 节律与睡眠调度
├── metrics.rs         # 六轴计算、工化指数
├── memory_lifecycle.rs # STM→LTM生命周期
├── fatigue.rs         # 疲劳值监测
├── file_loader.rs     # 七文件加载器
└── mod_system.rs      # DLC/Mod扩展
```

### 4.3 主引擎定义

```rust
use agent_diva_upsp_core::*;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UpspError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Not initialized")]
    NotInitialized,
}

pub type Result<T> = std::result::Result<T, UpspError>;

/// UPSP 主引擎
pub struct UpspEngine {
    persona_root: PathBuf,
    core: CoreAxes,
    mutation_rounds: CoreMutationRounds,
    state: StateJson,
    stm: Vec<StmEntry>,
    ltm_index: LtmIndex,
    config: UpspConfig,
}

impl UpspEngine {
    /// 创建新引擎
    pub fn new(persona_root: impl AsRef<Path>) -> Result<Self> {
        let root = persona_root.as_ref().to_path_buf();
        let engine = Self {
            persona_root: root,
            core: CoreAxes::new(),
            mutation_rounds: CoreMutationRounds::new(),
            state: StateJson::default(),
            stm: Vec::new(),
            ltm_index: LtmIndex::new(),
            config: UpspConfig::default(),
        };
        Ok(engine)
    }
    
    /// 从磁盘加载七文件
    pub fn load(&mut self) -> Result<()> {
        // 加载 core.md → self.core
        // 加载 state.json → self.state
        // 加载 STM.md → self.stm
        // 加载 LTM/index/*.json → self.ltm_index
        Ok(())
    }
    
    /// 保存到磁盘
    pub fn save(&self) -> Result<()> {
        // 保存 state.json
        // 保存 STM.md
        // 保存 LTM/index/*.json
        Ok(())
    }
    
    /// 执行一轮对话
    pub fn run_round(&mut self, llm_output: LlmOutput) -> Result<RoundResult> {
        // 1. 解析LLM输出的Δ值
        // 2. 更新动态六轴
        // 3. 执行热度结算
        // 4. 检查升格/遗忘
        // 5. 更新轮计数
        Ok(RoundResult::default())
    }
    
    /// 检查睡眠条件
    pub fn check_sleep_condition(&self) -> Option<SleepReason> {
        // 检查疲劳值双阈值
        // 返回睡眠原因
        None
    }
    
    /// 执行睡眠流程
    pub fn perform_sleep(&mut self) -> Result<()> {
        // 完整记忆压缩
        // 生成日志
        // 生成将来时规划
        // 写入快照
        // 重置状态
        Ok(())
    }
}

/// LLM 输出
pub struct LlmOutput {
    pub dynamic_delta: DynamicDelta,
    pub new_stm_entry: Option<StmEntry>,
    pub round_log: String,
}

/// 轮结果
#[derive(Debug, Default)]
pub struct RoundResult {
    pub new_state: Option<StateJson>,
    pub stm_changes: Vec<StmChange>,
    pub ltm_changes: Vec<LtmChange>,
    pub sleep_triggered: bool,
}

/// 睡眠原因
#[derive(Debug, Clone)]
pub enum SleepReason {
    ForcedByTime,
    ForcedByChars,
    Voluntary,
}
```

### 4.4 核心计算逻辑

#### metrics.rs - 六轴变化计算

```rust
use agent_diva_upsp_core::*;

/// 计算核心轴变化量
/// 公式: 变化量 = 核心变轮值 × (1 - |当前值|/100)
pub fn calculate_core_mutation(current_value: i16, mutation_round: u8) -> i16 {
    let magnitude = (mutation_round as f32) * (1.0 - (current_value.abs() as f32) / 100.0);
    magnitude.round() as i16
}

/// 应用核心轴变化
pub fn apply_core_mutation(axis: &mut i16, delta: i16) {
    let new_value = *axis + delta;
    *axis = new_value.clamp(-100, 100);
}

/// 计算动态轴实际变化
/// 公式: 实际变化量 = min(|Δ|, drift) × sign(Δ)
pub fn calculate_dynamic_change(delta: i16, drift: u8) -> i16 {
    let max_change = drift as i16;
    let clamped = delta.clamp(-max_change, max_change);
    if delta != 0 && clamped == 0 {
        delta.signum() * max_change
    } else {
        clamped
    }
}

/// 计算工化指数
pub fn calculate_workhood_index(
    self_reference: u8,
    self_reflection: u8,
    autonomy: u8,
) -> f32 {
    // 任一维度为0时工化指数归零
    if self_reference == 0 || self_reflection == 0 || autonomy == 0 {
        return 0.0;
    }
    
    let s_ref = self_reference as f32;
    let s_reflect = self_reflection as f32;
    let auto = autonomy as f32;
    
    // 标准化到0-1范围
    let s_ref_n = s_ref / 100.0;
    let s_reflect_n = s_reflect / 100.0;
    let auto_n = auto / 100.0;
    
    // 复合几何平均
    let product = s_ref_n * s_reflect_n * auto_n;
    if product <= 0.0 {
        0.0
    } else {
        (product.powf(1.0/3.0) * 100.0).round()
    }
}
```

#### memory_lifecycle.rs - 记忆生命周期

```rust
use agent_diva_upsp_core::*;

/// STM 热度衰减
pub fn decay_stm_heat(entry: &mut StmEntry, current_round: usize) -> MemoryFlow {
    // 每轮衰减规则
    // H >= 70: 减5
    // 40 <= H < 70: 减10
    // H < 40: 减15
    entry.heat = match entry.heat as i32 {
        h if h >= 70 => entry.heat - 5.0,
        h if h >= 40 => entry.heat - 10.0,
        _ => entry.heat - 15.0,
    };
    entry.heat = entry.heat.max(0.0);
    
    // 判断流向
    determine_memory_flow(entry)
}

/// 判断记忆流向
fn determine_memory_flow(entry: &StmEntry) -> MemoryFlow {
    if entry.ah_high >= 5 {
        MemoryFlow::PromoteToLtm
    } else if entry.ah_low <= -3 {
        MemoryFlow::Compress
    } else if entry.ah_low <= -5 {
        MemoryFlow::Forget
    } else {
        MemoryFlow::Stay
    }
}

/// 记忆流向枚举
#[derive(Debug, Clone, Copy)]
pub enum MemoryFlow {
    Stay,
    PromoteToLtm,
    Compress,
    Forget,
}

/// 压缩记忆为摘要
pub fn compress_to_summary(entry: &StmEntry, max_chars: usize) -> String {
    // 提取关键信息
    // 生成摘要
    // 截断到限制
    format!("[S] {}", entry.summary.chars().take(max_chars).collect::<String>())
}

/// 计算LTM热度
/// 公式: H_ltm = (N / (N+k)) × 100 × e^(-λ×Δt)
pub fn calculate_ltm_heat(call_count: usize, days_since_call: f64) -> f32 {
    const K: f64 = 4.0;
    const LAMBDA: f64 = 0.001;
    
    let n = call_count as f64;
    let decay = (-LAMBDA * days_since_call).exp();
    let base = (n / (n + K)) * 100.0;
    (base * decay) as f32
}
```

#### fatigue.rs - 疲劳监测

```rust
use agent_diva_upsp_core::*;
use chrono::{DateTime, Utc, Duration};

/// 检查疲劳阈值
pub fn check_fatigue_threshold(
    state: &StateJson,
    current_time: DateTime<Utc>,
    config: &FatigueConfig,
) -> FatigueLevel {
    let hours_since_sleep = if let Some(last_end) = state.last_sleep_end {
        (current_time - last_end).num_hours() as f64
    } else {
        // 从未睡眠，从开机计算
        (current_time - state.meta.last_update).num_hours() as f64
    };
    
    let chars_since_log = state.fatigue.log_chars_since_last_log;
    
    // 双阈值检查
    let time_level = match hours_since_sleep as f64 >= config.time_sleep_hours {
        true => FatigueLevel::ForcedSleep,
        false if hours_since_sleep >= config.time_warning_hours => FatigueLevel::Warning,
        _ => FatigueLevel::Normal,
    };
    
    let char_level = match chars_since_log >= config.chars_sleep {
        true => FatigueLevel::ForcedSleep,
        false if chars_since_log >= config.chars_warning => FatigueLevel::Warning,
        _ => FatigueLevel::Normal,
    };
    
    // 任一达强制阈值均触发
    if time_level == FatigueLevel::ForcedSleep || char_level == FatigueLevel::ForcedSleep {
        FatigueLevel::ForcedSleep
    } else if time_level == FatigueLevel::Warning || char_level == FatigueLevel::Warning {
        FatigueLevel::Warning
    } else {
        FatigueLevel::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FatigueLevel {
    Normal,
    Warning,
    ForcedSleep,
}

pub struct FatigueConfig {
    pub time_warning_hours: f64,
    pub time_sleep_hours: f64,
    pub chars_warning: usize,
    pub chars_sleep: usize,
}

impl Default for FatigueConfig {
    fn default() -> Self {
        Self {
            time_warning_hours: 24.0,
            time_sleep_hours: 30.0,
            chars_warning: 49152,
            chars_sleep: 65536,
        }
    }
}
```

---

## 5. Crate 3: agent-diva-memory 适配层

### 5.1 新增依赖

```toml
# agent-diva-memory/Cargo.toml
[dependencies]
agent-diva-core = { path = "../agent-diva-core", version = "0.2.0" }
agent-diva-upsp-core = { path = "../agent-diva-upsp-core" }  # 新增
agent-diva-upsp-engine = { path = "../agent-diva-upsp-engine" }  # 新增
```

### 5.2 适配层结构

```
agent-diva-memory/src/
├── upsp_compat/
│   ├── mod.rs
│   ├── bridge.rs       # UPSP ↔ DIVA 类型转换
│   ├── injector.rs    # 将UPSP回忆注入到检索系统
│   └── hybrid.rs       # 混合记忆服务
└── ...（现有代码）
```

### 5.3 桥接实现

```rust
use agent_diva_upsp_core::*;
use crate::types::{MemoryRecord, MemoryDomain, DiaryEntry};
use crate::WorkspaceMemoryService;

/// UPSP 与 DIVA 记忆系统的桥接器
pub struct UpspMemoryBridge {
    upsp_engine: Arc<UpspEngine>,
    memory_service: Arc<WorkspaceMemoryService>,
}

impl UpspMemoryBridge {
    /// 将 UPSP 的回忆结果转为 MemoryRecord
    pub fn upsp_recall_to_memory_records(
        &self,
        ltm_records: Vec<LtmRecord>,
    ) -> Vec<MemoryRecord> {
        ltm_records
            .into_iter()
            .map(|record| MemoryRecord {
                id: record.id,
                timestamp: record.timestamp,
                domain: Self::ltm_tier_to_domain(record.tier),
                scope: MemoryScope::Workspace,
                title: record.title,
                summary: record.summary,
                content: record.content,
                tags: vec!["upsp".to_string()],
                source_refs: vec![],
                confidence: record.heat / 100.0,
            })
            .collect()
    }
    
    /// 将 DiaryEntry 转为 UPSP StmEntry
    pub fn diary_to_stm_entry(
        &self,
        entry: &DiaryEntry,
    ) -> StmEntry {
        StmEntry {
            id: format!("MEM-{}", entry.id),
            memory_type: MemoryType::Full,
            timestamp: entry.timestamp,
            entry_round: 0,
            daily_round: 0,
            title: entry.title.clone(),
            summary: entry.summary.clone(),
            content: entry.body.clone(),
            heat: 50.0,
            ah_high: 0,
            ah_low: 0,
            zone: String::new(),
            ltm_status: "未归档".to_string(),
            locked: false,
            dynamic_influence: DynamicInfluence::default(),
            relation_influence: RelationInfluence::default(),
            workhood_influence: WorkhoodInfluence::default(),
        }
    }
    
    /// 混合检索：UPSP引擎 + 现有语义检索
    pub async fn hybrid_recall(
        &self,
        query: &str,
        include_upsp: bool,
    ) -> Result<Vec<MemoryRecord>> {
        // 1. 调用现有语义检索
        let semantic_results = self.memory_service
            .recall_records_for_context(query, 5)?;
        
        if !include_upsp {
            return Ok(semantic_results);
        }
        
        // 2. 调用UPSP引擎回忆
        let upsp_results = self.upsp_engine
            .recall(query)?;
        let upsp_records = self.upsp_recall_to_memory_records(upsp_results);
        
        // 3. 合并去重
        let mut combined = semantic_results;
        for record in upsp_records {
            if !combined.iter().any(|r| r.id == record.id) {
                combined.push(record);
            }
        }
        
        Ok(combined)
    }
    
    fn ltm_tier_to_domain(tier: LtmTier) -> MemoryDomain {
        match tier {
            LtmTier::Active => MemoryDomain::Workspace,
            LtmTier::Forgotten => MemoryDomain::Workspace,
            LtmTier::Archive => MemoryDomain::Workspace,
            LtmTier::Pinned => MemoryDomain::SelfModel,
            LtmTier::Skills => MemoryDomain::Task,
        }
    }
}
```

---

## 6. 集成方案

### 6.1 最小侵入性集成

```rust
// agent-diva-cli 初始化
use agent_diva_upsp_engine::UpspEngine;
use agent_diva_memory::{WorkspaceMemoryService, UpspMemoryBridge};

fn main() {
    // 检查是否启用UPSP
    let config = load_config()?;
    
    let memory_service = WorkspaceMemoryService::new(&workspace);
    
    let bridge = if config.upsp_enabled {
        let upsp_engine = UpspEngine::new(workspace.join("upsp"))
            .expect("UPSP engine init");
        Some(UpspMemoryBridge::new(upsp_engine, memory_service.clone()))
    } else {
        None
    };
    
    // 正常运行
    run_agent_loop(memory_service, bridge);
}
```

### 6.2 上下文组装

```rust
// agent-diva-agent/src/context.rs
pub async fn build_identity_context(&self) -> String {
    if let Some(bridge) = &self.upsp_bridge {
        // 使用UPSP引擎
        bridge.hybrid_recall("identity core", true)
            .await
            .map(|records| format_recall_context(&records))
            .unwrap_or_default()
    } else {
        // 回退到现有SOUL注入
        self.load_soul_md()
    }
}
```

---

## 7. 开发时间估算

| 阶段 | 工作 | 时间 | 产出 |
|------|------|------|------|
| **0** | 设计upsp-core的数据结构 | 2天 | 七文件类型定义 + 单元测试 |
| **1** | 实现upsp-engine的计算逻辑 | 3-4天 | 六轴变化、热度衰减、睡眠判断 |
| **2** | 实现memory_lifecycle状态机 | 2-3天 | STM→LTM完整流转 |
| **3** | 集成到agent-diva-memory | 1-2天 | bridge模块 + 混合检索 |
| **4** | CLI/GUI集成 | 1天 | 端到端可用 |

**总计：9-12天**

---

## 8. 关键优势

| 优势 | 说明 |
|------|------|
| **独立可测试** | 每个crate可单独测试、发布、版本控制 |
| **松耦合** | UPSP引擎不依赖agent-diva-memory的实现细节 |
| **可重用** | upsp-core/upsp-engine可供其他项目使用 |
| **渐进式** | 可以只部署upsp-core（数据类型），不启动引擎 |
| **向后兼容** | 现有agent-diva-memory功能不受影响 |
| **理论完整性** | UPSP的文件系统、六轴、睡眠机制完整实现 |

---

*文档版本：v0.1 | 2026-04-03*
