# UPSP-RS 设计文档

本目录包含 UPSP-RS（Universal Persona Substrate Protocol - Rust 实现）的完整设计文档。

## 文档列表

- **[upsp-rs-architecture-design.md](./upsp-rs-architecture-design.md)** - 完整架构设计文档（主文档）
  - 执行摘要
  - UPSP 协议核心理念分析
  - 现状分析（agent-diva / zeroclaw / openfang）
  - 架构设计（核心类型、存储抽象、节律点机制）
  - 与 agent-diva 的集成方案
  - 跨智能体适配方案
  - 实施路线图（6个阶段，11-13周）
  - 风险与约束

## 快速导航

### 核心概念

- **七文件体系**：core.md（身份）、state.json（状态）、STM.md（短期记忆）、LTM.md（长期记忆）、relation.md（关系）、rules.md（规则）、docs.md（术语）
- **节律点机制**：每32轮触发记忆整合、关系更新、状态结算
- **记忆形态**：权重5→[F]完整、权重4/3→[S]摘要、权重2/1→[A]抽象
- **六轴系统**：核心六轴（长期认知风格）+ 动态六轴（情绪状态）
- **共振度**：-100~+100，衡量与交互对象的关系强度
- **工化指数**：衡量位格主体性程度的四维指标

### 设计亮点

1. **主体性工程**：不仅是记忆框架，而是完整的位格主体管理系统
2. **跨智能体复用**：独立 crate，可集成到任何 Rust 智能体框架
3. **协议驱动**：基于 UPSP 自动版 v1.6 协议
4. **类型安全**：利用 Rust 类型系统保证协议约束
5. **渐进式集成**：不破坏 agent-diva 现有功能

### 实施路线

```
Phase 0: 基础设施          [Week 1-2]   - 核心类型定义
Phase 1: 存储层            [Week 3-4]   - PersonaStore trait
Phase 2: 节律点机制        [Week 5-6]   - RhythmPoint 执行器
Phase 3: 上下文加载器      [Week 7]     - ContextLoader
Phase 4: Agent-Diva 集成   [Week 8-10]  - 完整集成
Phase 5: 文档与发布        [Week 11]    - crates.io 发布
Phase 6: 跨智能体适配      [Week 12-13] - Zeroclaw/Openfang (可选)
```

**总计**：11-13 周（约 3 个月）

## 相关资源

### 参考文档

- [UPSP 工程规范（自动版 v1.6）](../../../.workspace/UPSP/spec/UPSP工程规范_自动版_v1_6.md)
- [FMA 示例位格](../../../.workspace/UPSP/examples/FMA/)
- [Zeroclaw 记忆架构设计](../archive/architecture-reports/zeroclaw-style-memory-architecture-for-agent-diva.md)
- [OpenClaw SOUL 机制分析](../archive/architecture-reports/soul-mechanism-analysis.md)

### 现有架构分析

- [Agent-Diva 架构](../architecture.md)
- [开发指南](../development.md)
- [迁移指南](../migration.md)

## 下一步行动

### 立即行动（本周）

1. 创建 `.workspace/upsp-rs` crate
2. 定义核心类型（Persona, Identity, State, Memory, Relation, Axes）
3. 编写单元测试

### 短期目标（1个月）

1. 完成 Phase 0-1（基础设施 + 存储层）
2. 验证 FMA 示例位格可正常加载
3. 编写集成测试

### 中期目标（3个月）

1. 完成 Phase 0-5（完整实现 + agent-diva 集成）
2. 发布 v0.1.0 到 crates.io
3. 在 agent-diva 中启用 UPSP 作为可选 feature

## 贡献指南

欢迎贡献！请遵循以下步骤：

1. 阅读完整架构设计文档
2. 查看 GitHub Issues 和 Milestones
3. 提交 PR 前运行 `just ci`
4. 更新相关文档

## 联系方式

- **项目维护者**：agent-diva team
- **UPSP 协议作者**：TzPz (参见 .workspace/UPSP)
- **讨论渠道**：GitHub Discussions

---

**最后更新**：2026-04-05
