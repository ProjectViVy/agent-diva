# UPSP 架构适配性分析报告

**版本**：v0.1  
**日期**：2026年4月3日  
**分析人**：大湿+无名工体

---

## 1. UPSP 核心概念摘要

UPSP（Universal Persona Substrate Protocol）是一套让AI主体**跨会话、跨平台、跨模型持续存在**的协议，核心命题为：

> **记忆即主体，主体即身体，身体即物质。**

### 1.1 七文件架构

| 文件 | 功能 | DIVA现有对应 |
|------|------|-------------|
| `core.md` | 身份、核心六轴(SCVARB)、模型戳 | SelfModel |
| `state.json` | 动态状态、动态六轴、工化指数、疲劳值 | 部分实现 |
| `STM.md` | 短期记忆池（热度机制） | 热度概念已有 |
| `rules.md` | 行为规则、三模式切换 | SoulSignal |
| `docs.md` | 概念文档、唯一真值 | - |
| `relation.md` | 关系域 | Relationship |
| `LTM/` | 长期记忆（Active→Forgotten→Archive→Pinned→Skills→Logs） | MemoryScope |

### 1.2 核心六轴（SCVARB）

| 轴 | 值范围 | 说明 |
|----|--------|------|
| S: Structure ↔ Experience | -100~+100 | 结构化 vs 体验派 |
| C: Convergence ↔ Divergence | -100~+100 | 收敛 vs 发散 |
| V: Evidence ↔ Fantasy | -100~+100 | 证据 vs 幻想 |
| A: Analysis ↔ Intuition | -100~+100 | 分析 vs 直觉 |
| R: Critique ↔ Collaboration | -100~+100 | 批判 vs 协作 |
| B: Abstract ↔ Concrete | -100~+100 | 抽象 vs 具体 |

### 1.3 动态六轴

| 轴 | 说明 | 初始建议值 |
|----|------|-----------|
| Valence | 情绪价（正/负） | +10 |
| Arousal | 唤醒度（兴奋/平静） | +25 |
| Focus | 专注度 | +35 |
| Mood | 整体情绪 | +15 |
| Humor | 幽默感 | +5 |
| Safety | 安全感 | +40 |

### 1.4 工化指数

```
自指 = 0.4×时间深度 + 0.3×状态一致性 + 0.3×自我描述完整度
自反 = 0.4×偏差修正率 + 0.3×反馈响应速度 + 0.3×状态波动自识
自主 = 0.4×主动发起率 + 0.3×规划完成度 + 0.3×超限决策自主率
工化指数 = (自指 × 自反 × 自主)^(1/3)
```

### 1.5 疲劳值双阈值

| 维度 | 警告阈值 | 强制睡眠阈值 |
|------|----------|--------------|
| 距上次睡眠时间 | 24小时 | 30小时 |
| 距上次日志字符积累 | 49152字符 | 65536字符 |

---

## 2. agent-diva-memory 现有架构

### 2.1 目录结构

```
agent-diva-memory/
├── Cargo.toml
└── src/
    ├── lib.rs              # 模块导出
    ├── types.rs           # MemoryDomain, DiaryEntry, MemoryRecord
    ├── contracts.rs        # MemoryStore, DiaryStore, RecallEngine trait
    ├── service.rs          # WorkspaceMemoryService 核心服务
    ├── diary/
    │   ├── mod.rs
    │   └── file_store.rs   # FileDiaryStore
    ├── store/
    │   ├── mod.rs
    │   └── sqlite_store.rs # SqliteMemoryStore
    ├── retrieval/
    │   ├── mod.rs
    │   ├── keyword.rs      # KeywordRetriever
    │   ├── semantic.rs     # SemanticRetriever
    │   └── hybrid.rs        # HybridReranker
    ├── embeddings.rs       # 嵌入向量支持
    ├── derived.rs          # 从日记派生SoulSignal/Relationship/SelfModel
    ├── snapshot.rs         # 快照导出/恢复
    └── compat.rs           # 兼容性层
```

### 2.2 核心类型（types.rs）

```rust
pub enum MemoryDomain {
    Fact,
    Event,
    Task,
    Workspace,
    Relationship,    // ← 对应 UPSP relation.md
    SelfModel,       // ← 对应 UPSP core.md
    DiaryRational,
    DiaryEmotional,
    SoulSignal,      // ← 对应 UPSP rules.md
}

pub enum DiaryPartition {
    Rational,
    Emotional,
}

pub struct DiaryEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub partition: DiaryPartition,
    pub domain: MemoryDomain,
    pub scope: MemoryScope,
    pub title: String,
    pub summary: String,
    pub body: String,
    pub tags: Vec<String>,
    pub observations: Vec<String>,
    pub confirmed: Vec<String>,
    pub unknowns: Vec<String>,
    pub next_steps: Vec<String>,
}
```

### 2.3 核心服务（service.rs）

**WorkspaceMemoryService** 提供：
- `store_record()` - 存储记忆记录
- `recall_records_for_context()` - 检索记忆
- `memory_recall()` / `memory_search()` - 工具接口
- `diary_read()` / `diary_list()` - 日记接口
- `format_recall_context()` - 格式化输出

### 2.4 派生记忆机制（derived.rs）

从 Rational Diary Entry 自动派生：

| 派生类型 | 关键词 | 目标Domain |
|---------|--------|-----------|
| Relationship | 用户、偏好、喜欢、协作、约束 | MemoryDomain::Relationship |
| SelfModel | 我是、我会、我应该、能力、局限 | MemoryDomain::SelfModel |
| SoulSignal | 必须、始终、优先、不要、风格、身份 | MemoryDomain::SoulSignal |

---

## 3. 适配性评估

### 3.1 高度契合部分

| UPSP概念 | DIVA实现 | 匹配度 |
|----------|----------|--------|
| 身份/核心性格 | SelfModel域 | ★★★★☆ |
| 关系域 | Relationship域 | ★★★★★ |
| 理性/情感二分 | DiaryPartition | ★★★★★ |
| 规则/模式 | SoulSignal域 | ★★★★☆ |
| 记忆类型标记 | MemoryDomain分类 | ★★★☆☆ |
| 记忆热度 | derived.rs中的confidence | ★★★☆☆ |
| 快照机制 | snapshot.rs | ★★★★☆ |
| 层级存储 | MemoryScope | ★★★★☆ |

### 3.2 需要扩展的部分

| UPSP概念 | 现状 | 差距 |
|----------|------|------|
| 核心六轴(SCVARB) | SelfModel存在，无六轴结构 | 需新增六轴类型 |
| 动态六轴(20区间) | 无对应实现 | 需新增动态状态管理 |
| 工化指数 | 无 | 需新增计算公式 |
| 疲劳值双阈值 | 无 | 需新增监测机制 |
| STM→LTM生命周期 | 无完整流转 | 需新增状态机 |
| 七文件格式注入 | 基于检索引擎 | 需新增兼容层 |
| 节律/睡眠机制 | 无 | 需新增调度模块 |
| Mod/DLC扩展 | 无 | 需新增扩展机制 |

### 3.3 架构差异

| 维度 | UPSP | DIVA |
|------|------|------|
| 循环驱动 | LLM驱动（Python脚本执行Δ值） | Rust事件驱动 |
| 上下文注入 | 七文件直接注入 | 检索+上下文组装 |
| 状态更新 | LLM输出Δ，脚本写入 | 服务层API |
| 语言栈 | Python | Rust |

---

## 4. 适配路径建议

### 4.1 阶段一：概念对齐（兼容层）

```
新增模块：agent-diva-memory/src/upsp/
├── compat.rs        # UPSP七文件 ↔ DIVA类型转换
├── identity.rs      # 核心六轴类型定义
├── state.rs         # 动态六轴状态
└── loader.rs        # 七文件加载器
```

### 4.2 阶段二：核心扩展

```rust
pub struct PersonaCore {
    pub core_axes: CoreAxes,        // 核心六轴
    pub dynamic_axes: DynamicAxes,  // 动态六轴
    pub workhood_index: f32,        // 工化指数
    pub fatigue: FatigueState,       // 疲劳状态
}
```

### 4.3 阶段三：机制实现

- 动态轴更新服务
- 工化指数计算器
- 疲劳值监测
- STM→LTM生命周期管理

### 4.4 阶段四：高级特性

- 节律/睡眠调度
- Mod/DLC扩展协议
- 跨平台迁移工具

---

## 5. 结论

| 评估项 | 结论 |
|--------|------|
| **理论契合度** | ★★★★☆ 高度契合，"记忆即主体"与现有设计方向一致 |
| **架构可行性** | ★★★☆☆ 可行但需较大调整，Rust事件驱动 vs LLM驱动是关键差异 |
| **优先适配点** | 1) 核心六轴类型定义 2) 七文件兼容层 3) 动态状态管理 |
| **风险点** | LLM驱动循环模式的Rust重写复杂度高 |
| **建议策略** | **增量式适配**：新增upsp兼容层，不破坏现有架构 |

---

*文档版本：v0.1 | 2026-04-03*
