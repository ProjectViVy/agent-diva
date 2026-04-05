# UPSP 与 DIVA Soul 兼容性分析

**版本**：v0.1  
**日期**：2026年4月3日  
**目的**：分析UPSP与DIVA现有Soul模块的冲突点，并提出兼容性解决方案  

---

## 1. 现有 DIVA Soul 设计

### 1.1 SoulState（agent-diva-core/src/soul/mod.rs）

```rust
/// Runtime state for soul/bootstrap lifecycle.
pub struct SoulState {
    /// Timestamp when bootstrap was first seeded.
    pub bootstrap_seeded_at: Option<DateTime<Utc>>,
    /// Timestamp when bootstrap was marked as completed.
    pub bootstrap_completed_at: Option<DateTime<Utc>>,
}

/// Small persistence helper for soul lifecycle state.
pub struct SoulStateStore {
    path: PathBuf,
}

impl SoulStateStore {
    pub fn new(workspace: impl AsRef<Path>) -> Self { ... }
    pub fn load(&self) -> std::io::Result<SoulState> { ... }
    pub fn save(&self, state: &SoulState) -> std::io::Result<()> { ... }
    pub fn is_bootstrap_completed(&self) -> bool { ... }
    pub fn mark_bootstrap_seeded(&self) -> std::io::Result<()> { ... }
    pub fn mark_bootstrap_completed(&self) -> std::io::Result<()> { ... }
}
```

**存储位置**：`<workspace>/.agent-diva/soul-state.json`

### 1.2 AgentSoulConfig（agent-diva-core/src/config/schema.rs）

```rust
/// Soul/identity settings
pub struct AgentSoulConfig {
    /// Whether soul context injection is enabled.
    pub enabled: bool,
    
    /// Max characters loaded from each soul markdown file.
    pub max_chars: usize,
    
    /// Whether to notify user when soul files are updated.
    pub notify_on_change: bool,
    
    /// If true, BOOTSTRAP.md is only used until bootstrap is completed.
    pub bootstrap_once: bool,
    
    /// Rolling window in seconds for frequent soul-change hints.
    pub frequent_change_window_secs: u64,
    
    /// Minimum soul-changing turns in window to trigger hints.
    pub frequent_change_threshold: usize,
    
    /// Add boundary confirmation hint when SOUL.md changes.
    pub boundary_confirmation_hint: bool,
}
```

### 1.3 SoulContextSettings（agent-diva-agent/src/context.rs）

```rust
/// Runtime controls for soul prompt injection.
pub struct SoulContextSettings {
    pub enabled: bool,
    pub max_chars: usize,
    pub bootstrap_once: bool,
}
```

### 1.4 SoulGovernanceSettings（agent-diva-agent/src/agent_loop.rs）

```rust
/// Runtime soft-governance settings for soul evolution.
pub struct SoulGovernanceSettings {
    /// Rolling window in seconds for "frequent changes" hints.
    pub frequent_change_window_secs: u64,
    
    /// Minimum number of soul-changing turns in window to trigger hints.
    pub frequent_change_threshold: usize,
    
    /// Add a confirmation hint when SOUL.md changes.
    pub boundary_confirmation_hint: bool,
}
```

### 1.5 SoulContext（agent-diva-memory/src/derived.rs）

```rust
const SOUL_RULE_KEYWORDS: &[&str] = &[
    "必须", "始终", "优先", "不要", "禁止", "must", "always", "never",
];

const SOUL_IDENTITY_KEYWORDS: &[&str] = &[
    "风格", "语气", "身份", "人格", "透明", "规则", "原则", "中文", "前缀", "沟通", 
    "soul", "identity",
];
```

---

## 2. Soul 模块职责总结

DIVA Soul 的职责范围：

| 职责 | 实现位置 | 说明 |
|------|----------|------|
| Bootstrap生命周期 | SoulState | 一次性初始化时间戳 |
| 文件变化监测 | AgentLoop | 监测SOUL.md变化 |
| 软治理警告 | SoulGovernanceSettings | 频繁变化时提示 |
| 上下文注入 | ContextBuilder | 将SOUL.md注入LLM |
| SoulSignal派生 | derived.rs | 从日记提取规则信号 |

---

## 3. UPSP 设计要求

### 3.1 PersonaCore（UPSP新增）

```rust
/// 核心六轴
pub struct CoreAxes {
    pub structure_experience: i16,       // S: -100~+100
    pub convergence_divergence: i16,     // C
    pub evidence_fantasy: i16,           // V
    pub analysis_intuition: i16,         // A
    pub critique_collaboration: i16,     // R
    pub abstract_concrete: i16,          // B
}

/// 动态六轴
pub struct DynamicAxes {
    pub valence: AxisState,
    pub arousal: AxisState,
    pub focus: AxisState,
    pub mood: AxisState,
    pub humor: AxisState,
    pub safety: AxisState,
}

/// 状态JSON
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
```

### 3.2 UPSP 新增职责

| 职责 | 说明 |
|------|------|
| 核心六轴管理 | SCVARB六轴及其变轮 |
| 动态六轴追踪 | 每轮更新20区间的状态 |
| 工化指数计算 | 三维度复合几何平均 |
| 疲劳值监测 | 双阈值触发睡眠 |
| 节律/睡眠调度 | 轮数驱动+时间驱动 |
| STM→LTM生命周期 | 热度衰减、升格、压缩、遗忘 |

---

## 4. 冲突点分析

### 4.1 冲突矩阵

| 冲突点 | DIVA Soul | UPSP | 风险等级 | 影响 |
|--------|-----------|------|----------|------|
| **概念层级** | Bootstrap + 软治理 | 核心身份系统 | 🔴 高 | UPSP是Soul的超集 |
| **State存储** | soul-state.json (简单) | state.json (复杂) | 🟡 中 | 分离存储可解决 |
| **LLM输入** | SOUL.md注入 | core.md + state.json注入 | 🟡 中 | 选一个注入可解决 |
| **更新机制** | 文件变化监测 | LLM输出Δ值 | 🟡 中 | 双轨道可并存 |
| **生命周期** | Bootstrap→Live | 七轮循环 | 🔴 高 | UPSP完全覆盖 |
| **治理方式** | 被动警告 | 主动计算 | 🟢 低 | 无冲突，可并存 |

### 4.2 冲突根本原因

```
DIVA Soul        →  "身份的守门员" (设置+验证模式)
UPSP PersonaCore →  "身份的完整生命周期" (状态机+演化模式)
```

**类比理解：**
- **DIVA Soul** = 设置好初始SOUL.md，然后自动监测变化
- **UPSP** = 跟踪SOUL每一次微观变化，用数据记录演化历程

### 4.3 冲突详细说明

#### 冲突1：生命周期覆盖

**DIVA Soul**：
```
Bootstrap → SoulEstablished → Live
   (一次性)     (完成后锁定)    (正常运行)
```

**UPSP**：
```
Init → Round1 → Round2 → ... → Maintenance → Sleep → RoundN+1 → ...
              (循环往复，状态持续演化)
```

**问题**：UPSP的"轮"机制与Soul的"一次性"冲突

#### 冲突2：状态存储

**DIVA Soul**：
```json
// soul-state.json
{
  "bootstrap_seeded_at": "2026-03-15T10:00:00Z",
  "bootstrap_completed_at": "2026-03-15T10:30:00Z"
}
```

**UPSP**：
```json
// state.json
{
  "meta": { "total_round": 128, "daily_round": 14 },
  "core_speed_wheel": 42,
  "dynamic_axes": { ... },
  "workhood_index": { ... },
  "fatigue": { ... }
}
```

**问题**：两个state.json用途完全不同

#### 冲突3：更新驱动模式

**DIVA Soul**：
- 事件驱动：SOUL.md文件变化 → 触发警告
- 被动模式：不主动修改身份

**UPSP**：
- 轮数驱动：每轮执行六轴计算
- 主动模式：LLM输出Δ值 → 主动更新状态

---

## 5. 兼容性解决方案

### 5.1 解决方案A：UPSP作为Soul的演化版（推荐）

**核心思想**：
```
SoulState: 仅记录bootstrap时间戳（保持不变）
SoulGuardian: 监测文件变化+软治理（保持现有逻辑）
PersonaCore: UPSP身份系统（新增，平行存在）
```

**文件分离**：
```
workspace/
├── .agent-diva/
│   └── soul-state.json          ← DIVA Soul（保持不变）
│
├── upsp/                        ← UPSP（新增）
│   ├── persona/                 
│   │   ├── core.md              
│   │   ├── state.json           
│   │   ├── STM.md               
│   │   ├── rules.md             
│   │   ├── docs.md              
│   │   ├── relation.md          
│   │   └── LTM/                 
│   └── config.json              
│
└── memory/                      ← DIVA Memory
    └── ...
```

### 5.2 配置扩展

```rust
// agent-diva-core/src/config/schema.rs（扩展）
pub struct AgentSoulConfig {
    // 现有字段（保持不变）
    enabled: bool,
    max_chars: usize,
    notify_on_change: bool,
    bootstrap_once: bool,
    frequent_change_window_secs: u64,
    frequent_change_threshold: usize,
    boundary_confirmation_hint: bool,
    
    // 新增：UPSP支持（可选）
    #[serde(default)]
    upsp_enabled: bool,              // 是否启用UPSP引擎
    #[serde(default)]
    upsp_persona_path: Option<String>, // UPSP位格根目录
}
```

### 5.3 兼容层实现

```rust
// agent-diva-memory/src/upsp_compat/hybrid.rs

/// 混合身份提供者
pub struct HybridSoulProvider {
    /// DIVA Soul状态
    soul_store: SoulStateStore,
    
    /// UPSP引擎（可选）
    upsp_engine: Option<Arc<UpspEngine>>,
    
    /// DIVA记忆服务
    memory_service: Arc<WorkspaceMemoryService>,
}

impl HybridSoulProvider {
    /// 决定使用哪个版本的身份
    pub fn should_use_upsp(&self) -> bool {
        // 条件：
        // 1. Soul bootstrap已完成
        // 2. 配置启用UPSP
        // 3. UPSP引擎已初始化
        self.soul_store.is_bootstrap_completed()
            && self.config.upsp_enabled
            && self.upsp_engine.is_some()
    }
    
    /// 获取身份上下文
    pub fn get_identity_context(&self) -> String {
        if self.should_use_upsp() {
            // 使用UPSP引擎
            self.upsp_engine.as_ref()
                .unwrap()
                .render_identity_context()
        } else {
            // 回退到DIVA版本
            self.load_soul_md_context()
        }
    }
    
    /// 获取治理上下文
    pub fn get_governance_context(&self) -> GovernanceContext {
        if self.should_use_upsp() {
            // UPSP的主动治理
            GovernanceContext::Upsp(self.upsp_engine.as_ref().unwrap().get_governance())
        } else {
            // DIVA的软治理
            GovernanceContext::Soft(self.load_soul_governance())
        }
    }
}
```

### 5.4 上下文组装

```rust
// agent-diva-agent/src/context.rs

impl ContextBuilder {
    /// 构建身份上下文
    pub async fn build_identity_context(&self) -> String {
        if let Some(bridge) = &self.upsp_bridge {
            if bridge.should_use_upsp() {
                // 优先使用UPSP
                return bridge.get_identity_context().await;
            }
        }
        // 回退到现有SOUL注入
        self.load_soul_md().await
    }
    
    /// 构建治理上下文
    pub fn build_governance_context(&self) -> String {
        if let Some(bridge) = &self.upsp_bridge {
            if bridge.should_use_upsp() {
                return bridge.get_governance_context().render();
            }
        }
        // 现有软治理
        self.build_soul_change_warning()
    }
}
```

---

## 6. 迁移策略

### 6.1 双轨并行期（Phase 1-2）

```
┌─────────────────────────────────────────────────────────────┐
│                    DIVA Soul (保持运行)                      │
│  ├── soul-state.json                                        │
│  ├── SOUL.md 注入                                            │
│  └── SoulGuardian (软治理)                                  │
└─────────────────────────────────────────────────────────────┘
                            ↓ (可选启用)
┌─────────────────────────────────────────────────────────────┐
│                    UPSP Engine (新增)                        │
│  ├── upsp/persona/ (七文件)                                  │
│  ├── PersonaCore (六轴+动态)                                │
│  └── MemoryLifecycle (状态机)                               │
└─────────────────────────────────────────────────────────────┘
```

**特点**：
- 用户可选择是否启用UPSP
- 现有DIVA Soul完全不受影响
- UPSP作为"增强模式"存在

### 6.2 渐进迁移期（Phase 3）

```
迁移条件检查：
├── bootstrap已完成 ✓
├── UPSP_engine初始化成功 ✓
├── 用户明确启用UPSP ✓
└── 测试验证通过 ✓
        ↓
从 SoulContextSettings 切换到 UpspBridge
```

### 6.3 完全统一期（Phase 4）

```
v2.0 目标：
├── UPSP成为默认身份系统
├── SoulGuardian降级为"兼容模式"
└── SoulState保留用于迁移
```

---

## 7. 代码修改清单

### 7.1 agent-diva-core

| 文件 | 修改内容 | 风险 |
|------|----------|------|
| `src/soul/mod.rs` | **不做修改** | 🟢 无风险 |
| `src/config/schema.rs` | 新增 `upsp_enabled` 等字段 | 🟢 向后兼容 |

### 7.2 agent-diva-agent

| 文件 | 修改内容 | 风险 |
|------|----------|------|
| `src/context.rs` | 新增 `UpspBridge` 字段 | 🟡 中等风险 |
| `src/agent_loop.rs` | `SoulGovernanceSettings` 标记 deprecated | 🟢 无风险 |

### 7.3 agent-diva-memory

| 文件 | 修改内容 | 风险 |
|------|----------|------|
| `Cargo.toml` | 新增UPSP依赖 | 🟢 无风险 |
| `src/lib.rs` | 新增 `upsp_compat` 模块 | 🟢 无风险 |
| `src/service.rs` | **不做修改** | 🟢 无风险 |

---

## 8. 冲突避免清单

| 项目 | DIVA做法 | UPSP做法 | 冲突避免 |
|------|----------|---------|----------|
| **存储位置** | `.agent-diva/soul-state.json` | `upsp/persona/state.json` | ✅ 不同目录 |
| **初始化** | bootstrap-once | 七文件系统 | ✅ UPSP可选启用 |
| **监测** | 文件变化 | 状态机周期 | ✅ 两个系统平行 |
| **LLM注入** | SOUL.md | UPSP prompts | ✅ 选一个注入 |
| **类型定义** | SoulState (简单) | StateJson (复杂) | ✅ 不同结构 |
| **更新驱动** | 事件驱动 | 轮数驱动 | ✅ 异步或轮询 |

---

## 9. 最终建议

### 9.1 立即行动

1. **保护现有代码**：SoulState/SoulGovernanceSettings 保持不动
2. **独立开发UPSP**：创建新crate，不修改现有模块
3. **创建兼容层**：UpspBridge 在运行时选择版本

### 9.2 标记废弃

```rust
// agent-diva-agent/src/agent_loop.rs
#[deprecated(since = "0.3.0", note = "use UpspEngine instead")]
pub struct SoulGovernanceSettings {
    // ...
}
```

### 9.3 核心理念

```
UPSP 不是 Soul 的替代品，而是 Soul 的 "完整版本"

Keep DIVA Soul lightweight
Make UPSP an opt-in enhancement
```

### 9.4 迁移周期

| 阶段 | 时间 | 目标 |
|------|------|------|
| Phase 1 | 1-2月 | UPSP crate独立开发完成 |
| Phase 2 | 2-3月 | 可选启用，用户反馈收集 |
| Phase 3 | 3-6月 | 逐步迁移用户到UPSP |
| Phase 4 | 6-12月 | 完全统一为UPSP (v2.0) |

---

## 10. 结论

| 评估项 | 结论 |
|--------|------|
| **冲突严重性** | 🟡 **中等** — 概念层级冲突，但可分离 |
| **修改范围** | 🟢 **小** — agent-diva-core 仅需标记deprecated |
| **破坏性** | 🟢 **零** — 向后兼容，可完全选择关闭UPSP |
| **推荐策略** | **平行运行** — DIVA Soul保持运行，UPSP可选启用 |
| **迁移周期** | 6-12个月（给用户适应时间） |

---

*文档版本：v0.1 | 2026-04-03*
