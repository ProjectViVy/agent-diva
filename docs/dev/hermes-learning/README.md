# Hermes 自我学习机制集成 - README

> **版本**: v0.1.0-draft  
> **日期**: 2026-04-05  
> **状态**: 规划阶段

---

## 文档概览

本目录包含将 Hermes-Agent 的自我学习能力融入 agent-diva 的完整规划文档。

### 文档列表

1. **[00-executive-summary.md](./00-executive-summary.md)** - 执行摘要
   - 一句话总结
   - Hermes 核心能力概览
   - 兼容性分析
   - 推荐架构
   - 实施优先级
   - 关键决策点

2. **[01-hermes-capabilities.md](./01-hermes-capabilities.md)** - Hermes 能力详解
   - RL 训练闭环（Tinker-Atropos）
   - Trajectory 数据生成与压缩
   - 技能系统（程序性记忆）
   - 记忆系统（声明性知识）
   - 完整学习闭环
   - Rust 实现考虑

   - 兼容性分析总结
   - 融合架构设计
   - 迁移策略
   - 配置扩展
   - 测试策略

4. **[03-implementation-plan.md](./03-implementation-plan.md)** - 实施计划
   - 实施路线图（13-18 周）
   - 关键技术决策
   - 风险管理
   - 质量保证
   - 发布计划
   - 后续优化方向

---

## 快速导航

### 核心概念

**Hermes 自我学习机制**：
- RL 训练闭环（GRPO 算法）
- Trajectory 压缩（保护头尾，压缩中间）
- 技能系统（自动创建，安全扫描）
- 记忆系统（Built-in + External 双层）
- 完整学习闭环（用户交互 → 训练 → 模型改进）

- 七文件体系（core.md, state.json, STM.md, LTM.md, relation.md, rules.md, docs.md）
- 节律点机制（每 32 轮触发）
- 工化指数（主体性度量）
- 11-13 周实施路线

**融合架构**：

### 关键决策

✅ **已确认**：
1. 使用单一 SQLite 数据库（brain.db）
3. 调用外部 Python 脚本实现 Trajectory 压缩
4. Phase 4（RL 训练）作为可选功能

⚠️ **待决策**：
1. 是否实现 Skills Hub 集成
2. 是否支持外部记忆提供者插件
3. 是否实现多模态记忆支持

### 实施路线

```
Phase 3: 学习闭环          [Week 11-15] - Trajectory + 触发器 + 钩子
Phase 4: RL 训练（可选）   [Week 16-17] - RL 编排 + 格式转换
Phase 5: 迁移与发布        [Week 18-20] - 迁移工具 + 测试 + 文档
```

**总计**：13-18 周（约 3.5-4.5 个月）

---

## 与现有计划的关系



**关系**：
- Hermes 提供自我学习能力
- 两者通过适配器模式融合
- 共享 brain.db 数据库

**协同点**：
- 检索能力：共享 SQLite 索引
- 会话管理：history.json 由 SessionDB 提供
- 上下文构建：融合为统一加载器

**冲突点**：
- 触发机制：节律点 vs 上下文压缩
- 索引层：统一为 brain.db
- 抽象接口：适配器模式解决

### Hermes 集成分析

**位置**：`docs/dev/hermes-integration/00-current-architecture-analysis.md`

**关系**：
- 分析了 agent-diva 现有架构
- 识别了集成点和扩展点
- 提出了重构建议
- 本规划是其具体实施方案

---

## 开始使用

### 阅读顺序

**快速了解**（15 分钟）：
1. 阅读 [00-executive-summary.md](./00-executive-summary.md)
2. 查看推荐架构图
3. 了解实施优先级

**深入理解**（1 小时）：
1. 阅读 [01-hermes-capabilities.md](./01-hermes-capabilities.md)
3. 理解融合架构设计

**实施准备**（2 小时）：
1. 阅读 [03-implementation-plan.md](./03-implementation-plan.md)
2. 查看实施路线图
3. 了解风险管理和质量保证

### 参与贡献

**立即行动**（本周）：
1. 召开架构评审会议，确认融合方案
3. 细化统一的 MemoryProvider 接口设计

**短期目标**（1 个月）：
1. 完成 Phase 1（基础设施）
3. 验证 FMA 示例位格可正常加载

**中期目标**（3-4 个月）：
1. 完成 Phase 1-3（基础设施 + 适配器 + 学习闭环）
2. 实现端到端的自我学习能力
3. 发布 v0.1.0

---

## 相关资源

### 内部文档

- [Hermes 集成架构分析](../hermes-integration/00-current-architecture-analysis.md)
- [Agent-Diva 架构概览](../architecture.md)
- [开发指南](../development.md)

### 外部参考

- [Hermes-Agent GitHub](https://github.com/NousResearch/hermes-agent)
- [Hermes-Agent 文档](https://hermes-agent.nousresearch.com/docs/)
- [Tinker-Atropos](https://github.com/NousResearch/tinker-atropos)

---

## 常见问题

### Q1: 为什么要集成 Hermes 的自我学习机制？

**A**: agent-diva 当前只有简单的记忆整合（consolidation），缺乏持续学习和自我优化能力。Hermes 提供了完整的学习闭环：
- RL 训练闭环（模型改进）
- Trajectory 压缩（训练数据生成）
- 技能系统（程序性记忆）
- 记忆系统（声明性知识）

集成后，agent-diva 将具备从经验中持续改进的能力。


**A**: 有潜在冲突，但可以通过分层融合解决：
- **触发机制**：统一触发器，节律点负责记忆整合，上下文压缩负责会话摘要
- **索引层**：统一为 brain.db，分层查询


### Q3: 实施周期是多久？

**A**: 13-18 周（约 3.5-4.5 个月），分 5 个 Phase：
- Phase 1: 基础设施（4-6 周）
- Phase 2: 适配器层（3-4 周）
- Phase 3: 学习闭环（4-5 周）
- Phase 4: RL 训练（可选，2-3 周）
- Phase 5: 迁移与发布（2-3 周）

详见 [03-implementation-plan.md](./03-implementation-plan.md)。

### Q4: RL 训练是必须的吗？

**A**: 不是。RL 训练作为 Phase 4 可选功能，不影响主线。即使不实现 RL 训练，agent-diva 也能获得：
- 技能系统（自动创建和改进）
- Trajectory 保存和压缩（为未来训练做准备）

RL 训练可以在后续版本中实现。

### Q5: 如何保证数据安全？

**A**: 多重保障：
- **备份机制**：保留 JSONL 备份
- **回滚机制**：迁移工具支持回滚
- **双写模式**：Phase 2 同时写入旧系统和新系统
- **充分测试**：单元测试、集成测试、端到端测试


### Q6: Rust 实现 Trajectory 压缩会很复杂吗？

**A**: 初期使用外部 Python 脚本，降低开发成本：
- 复用 Hermes 现有实现
- Python 生态更适合 LLM 调用
- 后续可优化为纯 Rust 实现

详见 [01-hermes-capabilities.md](./01-hermes-capabilities.md) 的 Rust 实现考虑部分。

### Q7: 如何参与贡献？

**A**: 欢迎贡献！请遵循以下步骤：
1. 阅读完整规划文档
2. 查看 GitHub Issues 和 Milestones
3. 提交 PR 前运行 `just ci`
4. 更新相关文档

详见 [CONTRIBUTING.md](../../CONTRIBUTING.md)。

---

## 联系方式

- **项目维护者**：agent-diva team
- **Hermes-Agent 作者**：Nous Research
- **讨论渠道**：GitHub Discussions

---

## 更新日志

### v0.1.0-draft (2026-04-05)
- 初始规划文档
- 完成 Hermes 能力分析
- 完成实施计划

---

**文档版本**：v0.1.0-draft  
**最后更新**：2026-04-05
