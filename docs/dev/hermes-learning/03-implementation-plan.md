# 融合架构设计与实施计划

> **版本**: v0.1.0-draft  
> **日期**: 2026-04-05

---

## 1. 实施路线图（13-18 周）

### Phase 1：基础设施（4-6 周）

#### Week 1-2：UPSP-RS Phase 0-1

**目标**：实现 UPSP-RS 核心类型和存储层

**任务**：
1. 创建 `.workspace/upsp-rs` crate
2. 定义核心类型（Persona, Identity, State, Memory, Relation, Axes）
3. 实现 PersonaStore trait
4. 实现 FilesystemStore
5. 编写单元测试

**交付物**：
- `upsp-rs/src/core/` - 核心类型定义
- `upsp-rs/src/storage/` - 存储抽象
- `upsp-rs/tests/` - 单元测试
- 验证 FMA 示例位格可正常加载

#### Week 3-4：Hermes SessionDB

**目标**：实现 SQLite + WAL + FTS5 会话数据库

**任务**：
1. 创建 `agent-diva-core/src/session/db.rs`
2. 定义数据库 schema（sessions, messages, FTS5 索引）
3. 实现 SessionDB 结构体
4. 实现 CRUD 操作
5. 实现跨会话搜索
6. 编写单元测试

**交付物**：
- `agent-diva-core/src/session/db.rs` - SessionDB 实现
- `agent-diva-core/src/session/schema.sql` - 数据库模式
- `agent-diva-core/src/session/migration.rs` - JSONL → SQLite 迁移工具
- 性能基准测试报告

#### Week 5-6：统一 MemoryProvider 接口

**目标**：设计并实现统一的记忆提供者接口

**任务**：
1. 定义 MemoryProvider trait
2. 实现 BuiltinMemoryProvider（读取 MEMORY.md）
3. 重构 MemoryManager 支持多提供者
4. 实现 prefetch_all, sync_all, extract_all 协调器
5. 编写集成测试

**交付物**：
- `agent-diva-core/src/memory/provider.rs` - MemoryProvider trait
- `agent-diva-core/src/memory/builtin.rs` - BuiltinMemoryProvider
- `agent-diva-core/src/memory/manager.rs` - 重构后的 MemoryManager
- 接口文档

---

### Phase 2：适配器层（3-4 周）

#### Week 7-8：UpspMemoryProvider 适配器

**目标**：实现 UPSP 到 Hermes MemoryProvider 的适配器

**任务**：
1. 创建 `agent-diva-core/src/memory/upsp_adapter.rs`
2. 实现 UpspMemoryProvider 结构体
3. 实现 MemoryProvider trait 的所有方法
4. 实现节律点整合逻辑
5. 编写单元测试和集成测试

**交付物**：
- `agent-diva-core/src/memory/upsp_adapter.rs` - 适配器实现
- 单元测试覆盖率 > 80%
- 集成测试验证节律点机制

#### Week 9：HolographicMemoryProvider

**目标**：实现 Hermes 事实存储提供者

**任务**：
1. 创建 `agent-diva-core/src/memory/holographic.rs`
2. 实现事实存储和检索
3. 实现事实验证和更新
4. 实现工具模式（fact_feedback, search_facts）
5. 编写单元测试

**交付物**：
- `agent-diva-core/src/memory/holographic.rs` - 事实存储实现
- 工具 schema 定义
- 单元测试

#### Week 10：SkillMemoryProvider

**目标**：实现技能系统作为记忆提供者

**任务**：
1. 创建 `agent-diva-agent/src/skills/provider.rs`
2. 实现技能自动创建触发器
3. 实现技能 CRUD 操作
4. 实现安全扫描
5. 编写单元测试

**交付物**：
- `agent-diva-agent/src/skills/provider.rs` - 技能提供者
- `agent-diva-agent/src/skills/scanner.rs` - 安全扫描器
- 单元测试

---

### Phase 3：学习闭环（4-5 周）

#### Week 11-12：Trajectory 保存和压缩

**目标**：实现 Trajectory 数据生成和压缩

**任务**：
1. 创建 `agent-diva-core/src/trajectory/store.rs`
2. 实现 Trajectory 保存（ShareGPT 格式）
3. 实现 Trajectory 压缩（调用外部 Python 脚本）
4. 实现批处理和并行压缩
5. 编写单元测试

**交付物**：
- `agent-diva-core/src/trajectory/store.rs` - Trajectory 存储
- `agent-diva-core/src/trajectory/compressor.rs` - 压缩器
- `scripts/trajectory_compressor.py` - Python 压缩脚本
- 性能基准测试

#### Week 13：统一触发器

**目标**：实现节律点 + 上下文压缩统一触发器

**任务**：
1. 创建 `agent-diva-agent/src/consolidation/trigger.rs`
2. 实现 ConsolidationTrigger 结构体
3. 集成到 Agent Loop
4. 实现职责分工（节律点 vs 上下文压缩）
5. 编写集成测试

**交付物**：
- `agent-diva-agent/src/consolidation/trigger.rs` - 统一触发器
- 集成测试验证两种触发机制

#### Week 14-15：Agent Loop 钩子集成

**目标**：在 Agent Loop 中集成会话生命周期钩子

**任务**：
1. 定义 SessionHooks trait
2. 在 Agent Loop 关键点调用钩子
3. 集成 MemoryManager.prefetch_all()
4. 集成 MemoryManager.sync_all()
5. 编写端到端测试

**交付物**：
- `agent-diva-core/src/session/hooks.rs` - SessionHooks trait
- `agent-diva-agent/src/agent_loop.rs` - 集成钩子
- 端到端测试

---

### Phase 4：RL 训练集成（可选，2-3 周）

#### Week 16：RL 训练编排

**目标**：实现 RL 训练编排（调用外部 Python 进程）

**任务**：
1. 创建 `agent-diva-core/src/rl/trainer.rs`
2. 实现 RLTrainer 结构体
3. 实现训练启动、监控、停止
4. 集成 WandB 监控
5. 编写集成测试

**交付物**：
- `agent-diva-core/src/rl/trainer.rs` - RL 训练器
- 集成测试

#### Week 17：Trajectory 格式转换

**目标**：实现 Trajectory 格式转换（Rust → ShareGPT）

**任务**：
1. 实现 ShareGPT 格式序列化
2. 实现批量转换工具
3. 验证与 Hermes 格式兼容性
4. 编写单元测试

**交付物**：
- `agent-diva-core/src/trajectory/format.rs` - 格式转换
- 单元测试

---

### Phase 5：迁移与发布（2-3 周）

#### Week 18：数据迁移工具

**目标**：实现数据迁移工具

**任务**：
1. 实现 MEMORY.md → UPSP LTM.md 迁移
2. 实现 JSONL → SessionDB 迁移
3. 实现回滚机制
4. 编写迁移指南
5. 验证迁移工具

**交付物**：
- `agent-diva-migration/src/memory_to_upsp.rs` - 记忆迁移
- `agent-diva-migration/src/sessions_to_db.rs` - 会话迁移
- 迁移指南文档

#### Week 19：端到端测试

**目标**：端到端测试和性能优化

**任务**：
1. 编写端到端测试套件
2. 性能基准测试
3. 内存泄漏检测
4. 并发压力测试
5. 修复发现的问题

**交付物**：
- 端到端测试套件
- 性能测试报告
- Bug 修复

#### Week 20：文档与发布

**目标**：文档更新和版本发布

**任务**：
1. 更新用户指南
2. 更新开发文档
3. 编写迁移指南
4. 准备 CHANGELOG
5. 发布 v0.1.0

**交付物**：
- 完整文档
- CHANGELOG.md
- v0.1.0 release

---

## 2. 关键技术决策

### 2.1 数据库选择

**决策**：使用单一 SQLite 数据库（brain.db）

**理由**：
- 避免数据冗余
- 简化查询路径
- 统一事务管理
- WAL 模式支持并发读

**替代方案**：
- 分离数据库（sessions.db + memories.db）
- 使用 PostgreSQL（过度工程）

### 2.2 UPSP 集成方式

**决策**：适配器模式（UpspMemoryProvider）

**理由**：
- 保持 upsp-rs 独立性
- 不修改 upsp-rs 源码
- 易于维护和升级
- 符合开闭原则

**替代方案**：
- 在 upsp-rs 中直接实现 MemoryProvider（耦合度高）
- Fork upsp-rs 并修改（维护成本高）

### 2.3 Trajectory 压缩实现

**决策**：调用外部 Python 脚本

**理由**：
- 复用 Hermes 现有实现
- 避免重复开发
- Python 生态更适合 LLM 调用
- 降低初期开发成本

**替代方案**：
- 纯 Rust 实现（开发成本高）
- 使用 PyO3 嵌入 Python（复杂度高）

### 2.4 RL 训练集成

**决策**：Phase 4 可选，调用外部进程

**理由**：
- RL 训练不是核心功能
- 外部进程隔离风险
- 降低初期复杂度
- 后续可优化

**替代方案**：
- 深度集成（复杂度高）
- 不实现（缺失学习闭环）

---

## 3. 风险管理

### 3.1 技术风险

| 风险 | 等级 | 影响 | 缓解策略 | 负责人 |
|------|------|------|---------|--------|
| UPSP + Hermes 架构冲突 | 🔴 高 | 集成失败 | 分层融合，明确职责边界，PoC 验证 | 架构师 |
| 数据迁移失败 | 🟡 中 | 数据丢失 | 保留 JSONL 备份，实现回滚机制，充分测试 | 开发者 |
| 性能下降 | 🟡 中 | 用户体验差 | 使用 WAL 模式，实现索引优化，性能基准测试 | 开发者 |
| Rust 实现 Trajectory 压缩复杂度 | 🟡 中 | 开发延期 | 先调用外部脚本，后续优化 | 开发者 |
| RL 训练集成复杂度 | 🟠 中高 | 开发延期 | 作为可选 Phase，使用外部进程调用 | 开发者 |
| 并发冲突 | 🟡 中 | 数据不一致 | 统一写入路径，使用 SQLite 管理并发，WAL 模式 | 开发者 |

### 3.2 进度风险

| 风险 | 等级 | 影响 | 缓解策略 |
|------|------|------|---------|
| Phase 1 延期 | 🟡 中 | 整体延期 | 预留 buffer，优先核心功能 |
| Phase 4 延期 | 🟢 低 | 可选功能缺失 | 作为可选 Phase，不影响主线 |
| 测试不充分 | 🟡 中 | 质量问题 | 每个 Phase 都有测试要求，CI/CD 自动化 |
| 文档滞后 | 🟢 低 | 用户困惑 | 每个 Phase 都有文档交付物 |

### 3.3 资源风险

| 风险 | 等级 | 影响 | 缓解策略 |
|------|------|------|---------|
| 开发人员不足 | 🟡 中 | 进度延期 | 优先核心功能，Phase 4 可选 |
| 测试资源不足 | 🟡 中 | 质量问题 | 自动化测试，CI/CD |
| 文档资源不足 | 🟢 低 | 用户困惑 | 每个 Phase 都有文档要求 |

---

## 4. 质量保证

### 4.1 测试策略

**单元测试**：
- 覆盖率 > 80%
- 每个模块都有单元测试
- 使用 `cargo test` 运行

**集成测试**：
- 跨模块功能测试
- 端到端场景测试
- 使用 `cargo test --test integration` 运行

**性能测试**：
- 基准测试（`cargo bench`）
- 内存泄漏检测（`valgrind`）
- 并发压力测试（`tokio-test`）

**回归测试**：
- 每次 PR 都运行完整测试套件
- CI/CD 自动化
- 测试失败阻止合并

### 4.2 代码审查

**审查清单**：
- [ ] 代码符合 Rust 风格指南
- [ ] 单元测试覆盖率 > 80%
- [ ] 集成测试通过
- [ ] 文档完整
- [ ] 无 clippy 警告
- [ ] 性能基准测试通过

**审查流程**：
1. 开发者提交 PR
2. CI/CD 自动运行测试
3. 至少一位审查者批准
4. 合并到主分支

### 4.3 性能指标

**目标**：
- SessionDB 加载历史 < 100ms（50 条消息）
- MemoryProvider.prefetch < 200ms
- Trajectory 保存 < 50ms
- 节律点整合 < 5s
- 上下文压缩 < 10s

**监控**：
- 使用 `criterion` 进行基准测试
- 每个 Phase 都有性能测试
- 性能回归阻止合并

---

## 5. 发布计划

### 5.1 版本规划

**v0.1.0（MVP）**：
- Phase 1-3 完成
- 基础学习闭环
- UPSP + Hermes 融合
- 数据迁移工具

**v0.2.0（增强）**：
- Phase 4 完成（可选）
- RL 训练集成
- Skills Hub 集成
- 外部记忆提供者插件

**v1.0.0（稳定）**：
- 生产就绪
- 完整文档
- 性能优化
- 安全加固

### 5.2 发布检查清单

**代码质量**：
- [ ] 所有测试通过
- [ ] 覆盖率 > 80%
- [ ] 无 clippy 警告
- [ ] 性能基准测试通过

**文档**：
- [ ] 用户指南完整
- [ ] 开发文档完整
- [ ] 迁移指南完整
- [ ] CHANGELOG 更新

**安全**：
- [ ] 依赖审计通过
- [ ] 安全扫描通过
- [ ] 敏感数据脱敏

**兼容性**：
- [ ] 向后兼容
- [ ] 迁移工具可用
- [ ] 回滚机制有效

---

## 6. 后续优化方向

### 6.1 性能优化

**优化点**：
1. **提示缓存**：实现 Anthropic Prompt Caching
2. **并行查询**：SessionDB 和 MemoryStore 并行查询
3. **索引优化**：FTS5 索引优化，向量索引
4. **缓存策略**：LRU 缓存热点记忆

### 6.2 功能增强

**增强点**：
1. **Skills Hub 集成**：社区技能共享
2. **外部记忆提供者插件**：Honcho, OpenViking, Mem0
3. **多模态支持**：图像、音频记忆
4. **分布式训练**：多机 RL 训练

### 6.3 生态建设

**建设点**：
1. **插件系统**：第三方插件支持
2. **社区贡献**：技能、记忆提供者
3. **文档完善**：教程、示例、最佳实践
4. **工具链**：CLI 工具、GUI 工具

---

**文档版本**：v0.1.0-draft  
**最后更新**：2026-04-05
