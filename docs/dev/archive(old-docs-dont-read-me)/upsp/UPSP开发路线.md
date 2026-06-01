# UPSP 开发路线

**版本**：v0.1  
**日期**：2026年4月3日  
**目标**：将UPSP协议实现为独立Rust crate并集成到agent-diva  

---

## 1. 总体路线图

```
Phase 0: 基础架构 (Week 1-2)
    ├── 创建 agent-diva-upsp-core crate
    ├── 定义七文件数据类型
    └── 单元测试覆盖

Phase 1: 引擎核心 (Week 3-4)
    ├── 创建 agent-diva-upsp-engine crate
    ├── 实现六轴计算逻辑
    ├── 实现状态机基础
    └── 引擎集成测试

Phase 2: 记忆生命周期 (Week 5-7)
    ├── STM 热度衰减机制
    ├── LTM 层级流转
    ├── 工化指数计算
    └── 疲劳值监测

Phase 3: DIVA集成 (Week 8-9)
    ├── 创建 upsp_compat 适配层
    ├── 混合记忆检索
    ├── ContextBuilder 集成
    └── 端到端测试

Phase 4: 高级特性 (Week 10-12)
    ├── 节律/睡眠调度
    ├── Mod/DLC扩展系统
    └── 文档与发布
```

---

## 2. Phase 0: 基础架构

### 2.1 目标

- 创建 `agent-diva-upsp-core` crate
- 定义UPSP七文件对应的Rust数据类型
- 实现基础的序列化/反序列化
- 编写完整的单元测试

### 2.2 交付物

```
agent-diva-upsp-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── core.rs           # CoreAxes, ModelStamp
    ├── state.rs          # DynamicAxes, StateJson, FatigueState
    ├── memory.rs         # StmEntry, LtmRecord, LtmTier
    ├── relation.rs       # RelationVector
    ├── diary.rs          # DiaryEntry
    ├── rules.rs          # Rules
    ├── config.rs         # Config
    └── validation.rs     # 规范校验
```

### 2.3 详细任务

| 任务 | 说明 | 预计时间 |
|------|------|----------|
| T0.1 | 创建crate目录结构和Cargo.toml | 0.5h |
| T0.2 | 实现 CoreAxes 结构体 | 2h |
| T0.3 | 实现 DynamicAxes 结构体 | 2h |
| T0.4 | 实现 StateJson 主结构 | 3h |
| T0.5 | 实现 MemoryDomain/Tier 类型 | 2h |
| T0.6 | 实现 StmEntry/LtmRecord | 3h |
| T0.7 | 实现 RelationVector | 2h |
| T0.8 | 实现 DiaryEntry | 2h |
| T0.9 | 实现 Rules 结构 | 1h |
| T0.10 | 实现 Config 结构 | 1h |
| T0.11 | 编写单元测试 | 3h |
| T0.12 | 编写文档注释 | 2h |

**小计：约 24 小时（3个工作日）**

### 2.4 验收标准

```rust
// 验收测试示例
#[test]
fn test_core_axes_serialization() {
    let axes = CoreAxes::new();
    let json = serde_json::to_string(&axes).unwrap();
    let parsed: CoreAxes = serde_json::from_str(&json).unwrap();
    assert_eq!(axes.persona_code(), parsed.persona_code());
}

#[test]
fn test_state_json_validation() {
    let state = StateJson::default();
    assert!(state.validate().is_ok());
}
```

---

## 3. Phase 1: 引擎核心

### 3.1 目标

- 创建 `agent-diva-upsp-engine` crate
- 实现六轴变化计算
- 实现动态轴更新逻辑
- 实现引擎主循环

### 3.2 交付物

```
agent-diva-upsp-engine/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── engine.rs          # UpspEngine
    ├── metrics.rs         # 六轴计算
    ├── state_machine.rs   # 状态转移
    ├── loader.rs          # 七文件加载
    └── config.rs          # 引擎配置
```

### 3.3 详细任务

| 任务 | 说明 | 预计时间 |
|------|------|----------|
| T1.1 | 创建crate目录结构和Cargo.toml | 0.5h |
| T1.2 | 实现 UpspEngine 主结构 | 4h |
| T1.3 | 实现 calculate_core_mutation | 2h |
| T1.4 | 实现 calculate_dynamic_change | 2h |
| T1.5 | 实现 apply_mutation 方法 | 2h |
| T1.6 | 实现 run_round 主循环 | 4h |
| T1.7 | 实现 load/save 方法 | 3h |
| T1.8 | 实现七文件加载器 | 4h |
| T1.9 | 编写集成测试 | 4h |

**小计：约 26 小时（3-4个工作日）**

### 3.4 核心算法

```rust
// 六轴变化公式
// 变化量 = 核心变轮值 × (1 - |当前值|/100)
pub fn calculate_core_mutation(current: i16, rounds: u8) -> i16 {
    let magnitude = (rounds as f32) * (1.0 - (current.abs() as f32) / 100.0);
    magnitude.round() as i16
}

// 动态轴实际变化
// 实际变化量 = min(|Δ|, drift) × sign(Δ)
pub fn calculate_dynamic_change(delta: i16, drift: u8) -> i16 {
    let max_change = drift as i16;
    delta.clamp(-max_change, max_change)
}
```

---

## 4. Phase 2: 记忆生命周期

### 4.1 目标

- 实现STM热度衰减机制
- 实现LTM层级流转（Active→Forgotten→Archive）
- 实现工化指数计算
- 实现疲劳值监测

### 4.2 交付物

```
agent-diva-upsp-engine/src/
├── memory_lifecycle.rs    # STM→LTM状态机
├── fatigue.rs            # 疲劳值监测
├── workhood.rs           # 工化指数
└── scheduler.rs          # 节律调度
```

### 4.3 详细任务

| 任务 | 说明 | 预计时间 |
|------|------|----------|
| T2.1 | 实现 decay_stm_heat 热度衰减 | 3h |
| T2.2 | 实现 determine_memory_flow 流向判断 | 2h |
| T2.3 | 实现 promote_to_ltm 升格逻辑 | 3h |
| T2.4 | 实现 compress_memory 压缩逻辑 | 2h |
| T2.5 | 实现 calculate_ltm_heat LTM热度 | 2h |
| T2.6 | 实现 calculate_workhood_index 工化指数 | 3h |
| T2.7 | 实现 check_fatigue_threshold 疲劳检查 | 3h |
| T2.8 | 实现 SleepReason 睡眠触发 | 2h |
| T2.9 | 实现 perform_sleep 睡眠流程 | 4h |
| T2.10 | 编写记忆生命周期测试 | 4h |

**小计：约 28 小时（4个工作日）**

### 4.4 热度衰减规则

```rust
// 每轮衰减规则
match heat {
    h if h >= 70.0 => heat - 5.0,   // 显著区
    h if h >= 40.0 => heat - 10.0,  // 未定区
    _ => heat - 15.0,                // 衰减区
}

// 流向判断
if ah_high >= 5 { MemoryFlow::PromoteToLtm }
else if ah_low <= -3 { MemoryFlow::Compress }
else if ah_low <= -5 { MemoryFlow::Forget }
else { MemoryFlow::Stay }
```

---

## 5. Phase 3: DIVA集成

### 5.1 目标

- 创建 `upsp_compat` 适配层
- 实现UPSP与DIVA Memory的桥接
- 集成到 ContextBuilder
- 端到端测试验证

### 5.2 交付物

```
agent-diva-memory/src/
├── upsp_compat/
│   ├── mod.rs
│   ├── bridge.rs       # HybridSoulProvider
│   ├── injector.rs     # 上下文注入
│   └── hybrid.rs       # 混合检索
```

### 5.3 详细任务

| 任务 | 说明 | 预计时间 |
|------|------|----------|
| T3.1 | 更新 Cargo.toml 添加UPSP依赖 | 0.5h |
| T3.2 | 创建 upsp_compat 目录 | 0.5h |
| T3.3 | 实现 bridge.rs 桥接器 | 4h |
| T3.4 | 实现 diary_to_stm_entry 转换 | 2h |
| T3.5 | 实现 hybrid_recall 混合检索 | 3h |
| T3.6 | 实现 injector.rs 上下文注入 | 3h |
| T3.7 | 修改 ContextBuilder 集成 | 4h |
| T3.8 | 添加配置项 upsp_enabled | 2h |
| T3.9 | 编写端到端测试 | 4h |

**小计：约 24 小时（3个工作日）**

### 5.4 桥接器接口

```rust
pub struct UpspMemoryBridge {
    upsp_engine: Arc<UpspEngine>,
    memory_service: Arc<WorkspaceMemoryService>,
}

impl UpspMemoryBridge {
    /// 将UPSP回忆转为DIVA MemoryRecord
    pub fn upsp_recall_to_memory_records(
        &self,
        ltm_records: Vec<LtmRecord>,
    ) -> Vec<MemoryRecord> { ... }
    
    /// 混合检索
    pub async fn hybrid_recall(
        &self,
        query: &str,
        include_upsp: bool,
    ) -> Result<Vec<MemoryRecord>> { ... }
}
```

---

## 6. Phase 4: 高级特性

### 6.1 目标

- 实现节律/睡眠调度
- 实现Mod/DLC扩展系统
- 完善文档
- 准备发布

### 6.2 交付物

```
agent-diva-upsp-engine/src/
├── scheduler.rs          # 定时调度
├── mod_system.rs         # DLC扩展
└── manifest.rs           # Mod清单

文档：
├── README.md
├── EXAMPLES.md
└── API.md
```

### 6.3 详细任务

| 任务 | 说明 | 预计时间 |
|------|------|----------|
| T4.1 | 实现 Scheduler 定时任务 | 4h |
| T4.2 | 实现 CronTrigger 节律点 | 3h |
| T4.3 | 实现 ModSystem DLC加载 | 6h |
| T4.4 | 实现 Manifest 解析 | 3h |
| T4.5 | 编写 README.md | 2h |
| T4.6 | 编写 EXAMPLES.md | 3h |
| T4.7 | 更新 agent-diva README | 1h |
| T4.8 | 版本发布准备 | 2h |

**小计：约 24 小时（3个工作日）**

---

## 7. 时间总览

| Phase | 任务数 | 预计时间 | 累计 |
|-------|--------|----------|------|
| Phase 0 | 12 | 24h | 24h |
| Phase 1 | 9 | 26h | 50h |
| Phase 2 | 10 | 28h | 78h |
| Phase 3 | 9 | 24h | 102h |
| Phase 4 | 8 | 24h | 126h |
| **总计** | **48** | **126h** | **~16天** |

---

## 8. 里程碑

| 里程碑 | 日期 | 验收标准 |
|--------|------|----------|
| M0 | Week 2 | `cargo test -p agent-diva-upsp-core` 全部通过 |
| M1 | Week 4 | `cargo test -p agent-diva-upsp-engine` 核心测试通过 |
| M2 | Week 7 | 记忆生命周期完整流程测试通过 |
| M3 | Week 9 | `just test` 全部通过，无回归 |
| M4 | Week 12 | 文档完整，版本发布 v0.1.0 |

---

## 9. 风险与应对

| 风险 | 影响 | 应对措施 |
|------|------|----------|
| Soul模块冲突 | 🟡 中 | 保持Soul不动，UPSP可选启用 |
| 性能问题 | 🟡 中 | 使用 `cargo bench` 性能测试 |
| 复杂度过高 | 🟡 中 | 分阶段交付，每阶段可运行 |
| LLM集成难度 | 🔴 高 | Phase 3专门处理，预留缓冲 |
| 测试覆盖不足 | 🟡 中 | TDD开发，测试先行 |

---

## 10. 资源需求

| 资源 | 数量 | 说明 |
|------|------|------|
| 开发时间 | 16人日 | ~3周全职开发 |
| 测试环境 | 1套 | 本地Rust环境 |
| 代码审查 | 2-3次 | 每个Phase结束时 |
| 文档撰写 | 8h | 分散在各Phase |

---

## 11. 依赖关系

```
Phase 0 (无依赖)
    ↓
Phase 1 (依赖 Phase 0)
    ↓
Phase 2 (依赖 Phase 1)
    ↓
Phase 3 (依赖 Phase 0, 1, 2)
    ↓
Phase 4 (依赖 Phase 3)
```

---

## 12. 下一步行动

### 立即开始（本周）

1. ✅ 创建 `agent-diva-upsp-core` 目录结构
2. ✅ 实现 `CoreAxes` 结构体
3. ✅ 实现 `DynamicAxes` 结构体
4. ✅ 编写单元测试

### 第二周

1. 实现 `StateJson` 主结构
2. 实现 `StmEntry` / `LtmRecord`
3. 完成 Phase 0 验收

### 第三周

1. 创建 `agent-diva-upsp-engine`
2. 实现引擎主结构
3. 实现六轴计算

---

*文档版本：v0.1 | 2026-04-03*
