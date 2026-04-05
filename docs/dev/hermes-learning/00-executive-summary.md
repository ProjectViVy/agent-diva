# Hermes 自我学习机制集成规划 - 执行摘要

> **版本**: v0.1.0-draft  
> **日期**: 2026-04-05  
> **作者**: Agent Diva Team

---

## 一句话总结

将 Hermes-Agent 的自我学习能力（RL 训练、trajectory 压缩、技能系统、记忆提供者）融入 agent-diva，同时协调现有的 UPSP 改造计划，构建一个具有持续学习和自我优化能力的 Rust 智能体框架。

---

## 核心问题

**我们要解决什么问题？**

1. **agent-diva 缺乏自我学习能力**：当前只有简单的记忆整合（consolidation），无法从经验中持续改进
2. **Hermes 的自学习机制如何适配 Rust 架构**：Hermes 是 Python 实现，agent-diva 是 Rust，需要架构适配
3. **UPSP 与 Hermes 的协调**：两者都涉及记忆系统改造，需要避免冲突并发挥协同效应

---

## Hermes 核心能力概览

### 1. RL 训练闭环（Tinker-Atropos）
- **GRPO 算法**：Group Relative Policy Optimization，无需单独 reward model
- **三进程协同**：Atropos API + Tinker trainer + Environment
- **环境发现**：AST 扫描动态加载任务定义
- **WandB 监控**：实时跟踪训练指标

### 2. Trajectory 数据生成与压缩
- **保护头尾策略**：保留首尾关键 turns，压缩中间区域
- **LLM 摘要**：用单个 summary 消息替换压缩区域
- **并行处理**：异步 API 调用 + Semaphore 限流
- **Token 预算控制**：目标 15250 tokens，摘要 750 tokens

### 3. 技能系统（程序性记忆）
- **自动创建触发**：复杂任务（5+ tool calls）成功后
- **技能操作**：create, patch, edit, delete, write_file, remove_file
- **安全扫描**：检查数据泄露、prompt injection、破坏性命令
- **渐进式披露**：Level 0（列表）→ Level 1（完整内容）→ Level 2（参考文件）

### 4. 记忆系统（声明性知识）
- **Built-in provider**：MEMORY.md, USER.md（始终活跃）
- **External provider**：Honcho, OpenViking, Mem0 等（最多一个）
- **自动化流程**：prefetch → sync → extract → mirror
- **Honcho 特色**：Dialectic Q&A，跨会话用户建模

### 5. 完整学习闭环
```
用户交互 → Agent 执行 → Trajectory 保存 → 
复杂任务完成 → Skill 创建 → 
会话结束 → Memory 提取 → 
Trajectory 压缩 → RL 训练 → 
模型改进 → 下次交互更好
```

---

## UPSP 改造计划概览

### 核心理念
- **位格主体管理**：不仅是记忆框架，而是完整的主体性工程
- **七文件体系**：core.md, state.json, STM.md, LTM.md, relation.md, rules.md, docs.md
- **节律点机制**：每 32 轮触发记忆整合、关系更新、状态结算
- **工化指数**：衡量位格主体性程度的四维指标

### 实施路线（11-13 周）
- Phase 0-1：基础设施 + 存储层（4 周）
- Phase 2：节律点机制（2 周）
- Phase 3：上下文加载器（1 周）
- Phase 4：Agent-Diva 集成（3 周）
- Phase 5：文档与发布（1 周）

---

## 兼容性分析

### ✅ 协同点（高度兼容）

1. **记忆存储层面**
   - UPSP：七文件体系（STM.md + LTM.md）
   - Hermes：MemoryProvider 抽象 + HolographicMemoryProvider
   - **协同**：UPSP 作为 MemoryProvider 的一种实现

2. **检索能力**
   - UPSP Phase 2：混合检索（关键词+语义+时间）+ SQLite 索引
   - Hermes：SessionDB（SQLite + FTS5）
   - **协同**：共享同一套索引基础设施

3. **会话管理**
   - UPSP：节律点机制 + history.json
   - Hermes：SessionDB + 会话生命周期钩子
   - **协同**：history.json 由 SessionDB 提供

4. **上下文构建**
   - UPSP：ContextLoader + 按权重召回
   - Hermes：MemoryLoader + 主动召回（3~7 条）
   - **协同**：融合为统一的上下文加载器

### ⚠️ 潜在冲突点

1. **记忆存储格式冲突**
   - UPSP：完全替代 MEMORY.md，使用七文件
   - Hermes：保留 MEMORY.md 作为 BuiltinMemoryProvider
   - **解决**：UPSP 的 STM.md/LTM.md 替代 MEMORY.md，BuiltinMemoryProvider 读取 UPSP 文件

2. **consolidation 触发机制冲突**
   - UPSP：节律点（每 32 轮）
   - Hermes：上下文压缩（50% 窗口）
   - **解决**：统一触发器，节律点负责记忆整合，上下文压缩负责会话摘要

3. **索引层职责冲突**
   - UPSP Phase 2：agent-diva 侧自建索引层
   - Hermes：SessionDB（SQLite + FTS5）
   - **解决**：使用单一 SQLite 数据库（brain.db），分层查询

4. **MemoryProvider 抽象冲突**
   - UPSP：upsp-rs 仅提供序列化
   - Hermes：定义 MemoryProvider trait
   - **解决**：适配器模式，实现 UpspMemoryProvider

### 🔴 架构层面的根本冲突

**记忆系统哲学差异**：
- UPSP：自上而下的主体设计（先有位格，再有记忆）
- Hermes：自下而上的能力堆叠（先有记忆，再有智能）

**解决方案 - 分层融合**：
```
应用层：UPSP 节律点 + Hermes 会话钩子
管理层：Hermes MemoryProvider 抽象 + UPSP MemoryManager
存储层：UPSP 七文件 + Hermes SessionDB
```

---

## 推荐架构：UPSP + Hermes 融合层

```
┌─────────────────────────────────────────────────────────────┐
│  应用层 (Agent Loop + Context Builder)                      │
│  - 会话生命周期钩子（Hermes）                                │
│  - 节律点触发器（UPSP）                                      │
│  - RL 训练编排（Hermes）                                     │
├─────────────────────────────────────────────────────────────┤
│  记忆管理层 (MemoryManager)                                 │
│  - UpspMemoryProvider（UPSP 适配器）                        │
│  - HolographicMemoryProvider（Hermes 事实存储）             │
│  - SkillMemoryProvider（技能系统）                          │
├─────────────────────────────────────────────────────────────┤
│  存储层                                                      │
│  - UPSP 七文件（core.md, state.json, STM.md, LTM.md, etc.）│
│  - SessionDB（SQLite + FTS5，Hermes）                       │
│  - brain.db（统一索引数据库）                               │
│  - Trajectory Store（训练数据）                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 实施优先级（13-18 周）

### Phase 1：基础设施（4-6 周）
1. 实现 UPSP-RS Phase 0-1（核心类型 + 存储层）
2. 实现 Hermes SessionDB（SQLite + WAL + FTS5）
3. 设计统一的 MemoryProvider 接口

### Phase 2：适配器层（3-4 周）
4. 实现 UpspMemoryProvider 适配器
5. 实现 HolographicMemoryProvider
6. 实现 SkillMemoryProvider（技能系统）
7. 重构 MemoryManager 支持多提供者

### Phase 3：学习闭环（4-5 周）
8. 实现 Trajectory 保存和压缩（Rust 实现）
9. 实现技能自动创建触发器
10. 实现节律点 + 上下文压缩统一触发器
11. 实现 Agent Loop 钩子集成

### Phase 4：RL 训练集成（可选，2-3 周）
12. 实现 RL 训练编排（调用外部 Python 进程）
13. 实现 Trajectory 格式转换（Rust → ShareGPT）
14. 集成 WandB 监控

### Phase 5：迁移与发布（2-3 周）
15. 实现数据迁移工具（JSONL + MEMORY.md → UPSP + SessionDB）
16. 端到端测试 + 性能优化
17. 文档更新 + 用户指南
18. 发布 v0.1.0

---

## 关键决策点

### 必须决策
1. ✅ **接受 UPSP 完全替代 MEMORY.md**（建议：是，但保留过渡期）
2. ✅ **使用统一的 SQLite 数据库**（建议：是，避免数据冗余）
3. ✅ **使用适配器模式集成 UPSP**（建议：是，保持 upsp-rs 独立性）

### 可选决策
4. ⚠️ **是否实现 RL 训练集成**（建议：Phase 4 可选，先完成基础闭环）
5. ⚠️ **是否实现 Skills Hub 集成**（建议：后续版本，先实现本地技能系统）
6. ⚠️ **是否支持外部记忆提供者插件**（建议：后续版本，先完成内置提供者）

---

## 风险评估

| 风险 | 等级 | 缓解策略 |
|------|------|---------|
| UPSP + Hermes 架构冲突 | 🔴 高 | 分层融合，明确职责边界 |
| 数据迁移失败 | 🟡 中 | 保留 JSONL 备份，实现回滚机制 |
| 性能下降 | 🟡 中 | 使用 WAL 模式，实现索引优化 |
| Rust 实现 Trajectory 压缩复杂度 | 🟡 中 | 先实现简单版本，后续优化 |
| RL 训练集成复杂度 | 🟠 中高 | 作为可选 Phase，使用外部进程调用 |

---

## 下一步行动

### 立即行动（本周）
1. 召开架构评审会议，确认融合方案
2. 创建 PoC 验证 UpspMemoryProvider 适配器
3. 细化统一的 MemoryProvider 接口设计

### 短期目标（1 个月）
1. 完成 Phase 1（基础设施）
2. 实现 SessionDB 和 UPSP-RS Phase 0-1
3. 验证 FMA 示例位格可正常加载

### 中期目标（3-4 个月）
1. 完成 Phase 1-3（基础设施 + 适配器 + 学习闭环）
2. 实现端到端的自我学习能力
3. 发布 v0.1.0

---

## 相关文档

- [01-hermes-capabilities.md](./01-hermes-capabilities.md) - Hermes 能力详解
- [02-upsp-integration.md](./02-upsp-integration.md) - UPSP 集成方案
- [03-architecture-design.md](./03-architecture-design.md) - 融合架构设计
- [04-implementation-plan.md](./04-implementation-plan.md) - 实施计划
- [05-migration-guide.md](./05-migration-guide.md) - 数据迁移指南

---

**文档版本**：v0.1.0-draft  
**最后更新**：2026-04-05
