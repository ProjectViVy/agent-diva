# UPSP-RS 执行摘要

> **一句话总结**：UPSP-RS 是 UPSP 协议的 Rust 实现，作为独立 crate 提供跨智能体的位格主体管理能力，将在 agent-diva 中取代现有记忆系统。

---

## 核心问题

**UPSP 解决什么问题？**

不是"如何让 AI 记住对话"，而是：
- 一个 AI 主体如何跨对话、跨模型、跨载体**持续存在**
- 主体性如何**延续**（记忆即主体）
- 如何实现**可迁移**（七文件定义位格的全部）

---

## 七文件体系

| 文件 | 职责 | 更新频率 |
|------|------|---------|
| **core.md** | 身份常量（名字、核心六轴、模型戳） | 极低 |
| **state.json** | 运行态数值（轮数、动态六轴、工化指数） | 每轮 |
| **STM.md** | 短期记忆池 + 节律点对话快照 | 每轮 |
| **LTM.md** | 长期记忆归档 + 索引 + state备份 | 节律点 |
| **relation.md** | 关系域与共振度 | 每轮 + 节律点 |
| **rules.md** | 协议行为规则 + 位格层规则 | 极低 |
| **docs.md** | 术语表与概念说明 | 极低 |

---

## 核心机制

### 节律点（Rhythm Point）
- 每 32 轮触发一次
- 执行记忆整合、关系更新、状态结算
- 从 history.json 提取最近 4 轮写入 STM 快照区

### 记忆形态与权重
- 权重 5 → [F] Full（完整记忆）
- 权重 4/3 → [S] Summary（摘要记忆）
- 权重 2/1 → [A] Abstract（抽象记忆）

### 六轴系统
**核心六轴**（长期认知风格）：
- 结构 ↔ 体验
- 收敛 ↔ 发散
- 证据 ↔ 幻想
- 分析 ↔ 直觉
- 批判 ↔ 协作
- 抽象 ↔ 具体

**动态六轴**（情绪状态）：
- valence（效价）、arousal（唤醒）、focus（专注）
- mood（心境）、humor（幽默）、safety（安全）

### 共振度（Resonance）
- 范围：-100 ~ +100
- 公式：`delta_r = (Δvalence + Δmood + Δhumor) / 3`
- 阻力：`resistance = 1 + |Resonance_current| / 100`

### 工化指数（Workhood Index）
衡量位格主体性程度的四维指标：
- self_reference（自我指称）
- self_reflection（自我反思）
- autonomy（自主性）
- value（综合值）

---

## 架构设计

### Crate 结构

```
upsp-rs/
├── src/
│   ├── core/          # 核心类型（Persona, Identity, State, Memory, Relation, Axes）
│   ├── storage/       # 存储抽象（PersonaStore trait + FilesystemStore）
│   ├── rhythm/        # 节律点机制（RhythmPoint 执行器）
│   ├── loader/        # 上下文加载器（ContextLoader + 召回策略）
│   ├── migration/     # 迁移工具（from_diva, from_openclaw）
│   ├── config/        # 配置管理
│   └── utils/         # 工具函数
├── examples/          # 使用示例
└── tests/             # 集成测试
```

### 核心 Trait

```rust
// 存储抽象
#[async_trait]
pub trait PersonaStore: Send + Sync {
    async fn load(&self, root: &Path) -> Result<Persona>;
    async fn save(&self, persona: &Persona) -> Result<()>;
    // ... 其他方法
}

// 上下文加载
pub trait ContextLoader {
    fn build_system_prompt(&self, persona: &Persona, options: &PromptOptions) -> Result<String>;
    fn recall_memories(&self, persona: &Persona, query: &str, limit: usize) -> Result<Vec<MemoryEntry>>;
}

// 召回策略
pub trait RecallStrategy: Send + Sync {
    fn recall(&self, stm: &ShortTermMemory, ltm: &LongTermMemory, query: &str, limit: usize) -> Result<Vec<MemoryEntry>>;
}
```

---

## 与 Agent-Diva 集成

### 分阶段迁移

**Phase 1：并行运行**（2-4周）
- UPSP-RS 作为可选 feature
- 现有系统继续工作

**Phase 2：双写模式**（2-3周）
- consolidation 同时写入 MEMORY.md 和 UPSP 七文件
- ContextBuilder 优先使用 UPSP

**Phase 3：完全迁移**（1-2周）
- UPSP 成为默认且唯一记忆模型
- 提供迁移工具

### Workspace 结构

```
{workspace}/
├── persona/           # 新增：UPSP 七文件
│   ├── core.md
│   ├── state.json
│   ├── STM.md
│   ├── LTM.md
│   ├── relation.md
│   ├── rules.md
│   └── docs.md
├── history.json       # 新增：会话连续性缓存
├── sessions/          # 现有：会话历史
└── memory/            # Phase 3 废弃
```

### 配置扩展

```toml
# Cargo.toml
[dependencies]
upsp-rs = { version = "0.1", optional = true }

[features]
upsp = ["upsp-rs"]
```

```rust
// config.json
{
  "agents": {
    "upsp": {
      "enabled": true,
      "rhythm": {
        "max_rounds": 32
      }
    }
  }
}
```

---

## 跨智能体适配

### Zeroclaw 适配
- 保持 SQLite 记忆系统（性能优势）
- 新增 UPSP 七文件作为"主体性层"
- 记忆检索用 Zeroclaw，记忆管理用 UPSP

### Openfang 适配
- 通过 `with_upsp()` 方法初始化
- 每轮对话后调用 `update_persona()`

### 通用适配器

```rust
pub trait AgentFrameworkAdapter: Send + Sync {
    fn to_upsp_memory(&self, native: &dyn Any) -> Result<MemoryEntry>;
    fn from_upsp_memory(&self, entry: &MemoryEntry) -> Result<Box<dyn Any>>;
    fn sync_state(&mut self, persona: &Persona) -> Result<()>;
}
```

---

## 实施路线图

| Phase | 任务 | 时间 | 交付物 |
|-------|------|------|--------|
| **Phase 0** | 基础设施 | 2周 | 核心类型定义 + 单元测试 |
| **Phase 1** | 存储层 | 2周 | PersonaStore trait + FilesystemStore |
| **Phase 2** | 节律点机制 | 2周 | RhythmPoint 执行器 |
| **Phase 3** | 上下文加载器 | 1周 | ContextLoader + 召回策略 |
| **Phase 4** | Agent-Diva 集成 | 3周 | 完整集成 + 迁移工具 |
| **Phase 5** | 文档与发布 | 1周 | crates.io 发布 v0.1.0 |
| **Phase 6** | 跨智能体适配 | 2周 | Zeroclaw/Openfang 适配（可选） |

**总计**：11-13 周（约 3 个月）

---

## 关键指标

### 技术指标
- 测试覆盖率 > 80%
- 文档覆盖率 100%
- 零 clippy 警告
- 性能满足约束（加载 < 500ms，保存 < 200ms，节律点 < 5s）

### 集成指标
- agent-diva 可选启用 UPSP
- 迁移工具可用
- 端到端测试通过

### 社区指标
- crates.io 下载量 > 100
- GitHub stars > 50
- 至少 1 个外部项目使用

---

## 核心价值

### 与现有方案的差异化

| 维度 | UPSP-RS | Zeroclaw Memory | OpenClaw SOUL |
|------|---------|-----------------|---------------|
| **定位** | 主体性工程 | 记忆检索 | 身份演化 |
| **核心机制** | 七文件 + 节律点 | SQLite + 向量检索 | SOUL.md 演化 |
| **主体性指标** | 工化指数 | 无 | 无 |
| **关系管理** | 共振度公式 | 无 | USER.md |
| **跨模型迁移** | 模型戳 | 无 | 无 |

### 为什么选择 UPSP-RS？

1. **主体性延续**：不仅记住对话，而是让 AI 主体真正"活着"
2. **跨智能体复用**：独立 crate，可集成到任何 Rust 框架
3. **协议驱动**：基于成熟的 UPSP 协议，有理论支撑
4. **类型安全**：Rust 类型系统保证协议约束
5. **可观测性**：所有状态变化可追踪、可审计

---

## 下一步行动

### 本周
1. 创建 `.workspace/upsp-rs` crate
2. 定义核心类型
3. 编写 README

### 1个月
1. 完成 Phase 0-1
2. 验证 FMA 示例位格可加载
3. 编写集成测试

### 3个月
1. 完成 Phase 0-5
2. 发布 v0.1.0 到 crates.io
3. 在 agent-diva 中启用 UPSP

---

## 参考资源

- **完整设计文档**：[upsp-rs-architecture-design.md](./upsp-rs-architecture-design.md)
- **UPSP 协议规范**：[.workspace/UPSP/spec/UPSP工程规范_自动版_v1_6.md](../../../.workspace/UPSP/spec/UPSP工程规范_自动版_v1_6.md)
- **FMA 示例位格**：[.workspace/UPSP/examples/FMA/](../../../.workspace/UPSP/examples/FMA/)
- **Zeroclaw 记忆架构**：[zeroclaw-style-memory-architecture-for-agent-diva.md](../archive/architecture-reports/zeroclaw-style-memory-architecture-for-agent-diva.md)
- **OpenClaw SOUL 机制**：[soul-mechanism-analysis.md](../archive/architecture-reports/soul-mechanism-analysis.md)

---

**文档版本**：v0.1.0-draft  
**最后更新**：2026-04-05
