# Deep Interview Spec: UPSP Memory Integration for Agent-Diva

## Metadata
- Interview ID: upsp-memory-integration-2026-04-05
- Rounds: 6
- Final Ambiguity Score: 14%
- Type: brownfield
- Generated: 2026-04-05
- Threshold: 20%
- Status: PASSED

## Clarity Breakdown
| Dimension | Score | Weight | Weighted |
|-----------|-------|--------|----------|
| Goal Clarity | 0.90 | 0.35 | 0.315 |
| Constraint Clarity | 0.95 | 0.25 | 0.238 |
| Success Criteria | 0.75 | 0.25 | 0.188 |
| Context Clarity | 0.80 | 0.15 | 0.120 |
| **Total Clarity** | | | **0.861** |
| **Ambiguity** | | | **0.139 (14%)** |

## Goal

**在upsp-rs完成后，将agent-diva的memory系统完全迁移到UPSP格式，实现混合检索能力（关键词+语义+时间），废弃现有的MEMORY.md和HISTORY.md，构建agent-diva侧的索引层以支持跨会话检索。**

核心目标分解：
1. **完全替代Markdown存储** - UPSP成为唯一的长期记忆存储格式
2. **Consolidation输出侧集成** - 在consolidation LLM输出memory_update后转换为UPSP格式
3. **混合检索实现** - 构建关键词、语义、时间三维索引，支持智能检索
4. **索引层构建** - 因upsp-rs仅提供序列化能力，agent-diva需自建索引基础设施

## Constraints

### 技术边界
- **upsp-rs职责范围** - 仅负责UPSP格式的序列化/反序列化，不提供索引和查询能力
- **索引技术栈** - agent-diva需自建索引层：
  - SQLite用于关键词和时间范围查询
  - 向量数据库（qdrant/milvus/faiss）用于语义检索
  - Embedding模型用于生成语义向量
- **集成点** - Consolidation输出侧（memory_update → UPSP转换）
- **存储策略** - 完全替代，不保留Markdown文件

### 架构约束
- **现有组件改造**
  - MemoryManager: 从读写MEMORY.md/HISTORY.md改为读写UPSP格式
  - ContextBuilder: 从全量注入MEMORY.md改为调用检索层获取相关片段
  - Consolidation: 输出memory_update后增加UPSP转换步骤
- **新增组件**
  - UPSP Converter: 将memory_update转换为UPSP格式
  - Index Manager: 管理多维索引的构建和更新
  - Retrieval Layer: 实现混合检索逻辑
  - Vector Store: 存储和查询语义向量

### 数据流约束
```
Session messages (JSONL)
  ↓ (100条阈值触发)
Consolidation LLM
  ↓ (输出memory_update)
UPSP Converter ← upsp-rs (序列化)
  ↓
UPSP Storage (替代MEMORY.md/HISTORY.md)
  ↓ (同步)
Index Manager
  ├→ SQLite Index (关键词+时间)
  └→ Vector Store (语义)
  ↓ (查询)
Retrieval Layer
  ↓ (注入相关片段)
ContextBuilder → System Prompt → LLM
```

## Non-Goals

- **不保留Markdown格式** - 不维护MEMORY.md/HISTORY.md的双写或兼容层
- **不在upsp-rs侧实现索引** - 索引能力完全由agent-diva负责
- **不支持渐进迁移** - 立即废弃Markdown，全面切换到UPSP
- **不实现单一检索策略** - 必须支持混合检索（关键词+语义+时间）

## Acceptance Criteria

### Phase 1: UPSP集成基础
- [ ] upsp-rs库集成到agent-diva-core
- [ ] UPSP Converter实现：memory_update → UPSP格式转换
- [ ] Consolidation pipeline改造：在输出memory_update后调用UPSP Converter
- [ ] UPSP Storage实现：替代MemoryManager的MEMORY.md/HISTORY.md读写逻辑
- [ ] 验证：consolidation触发后，生成UPSP格式文件而非Markdown文件

### Phase 2: 索引层构建
- [ ] SQLite Index实现：
  - 关键词倒排索引
  - 时间戳索引
  - 支持AND/OR/NOT逻辑查询
- [ ] Vector Store选型和集成（qdrant/milvus/faiss）
- [ ] Embedding模型集成（选择本地模型或API）
- [ ] Index Manager实现：
  - 监听UPSP Storage变化
  - 自动更新SQLite和Vector索引
  - 保证索引与UPSP文件的一致性
- [ ] 验证：新增memory后，索引自动更新且可查询

### Phase 3: 混合检索实现
- [ ] Retrieval Layer实现：
  - 关键词检索接口
  - 语义检索接口（基于embedding相似度）
  - 时间范围检索接口
  - 混合检索策略（多维度打分和排序）
- [ ] ContextBuilder改造：
  - 移除全量MEMORY.md注入逻辑
  - 调用Retrieval Layer获取相关memory片段
  - 根据对话上下文动态选择检索策略
- [ ] 验证：
  - 关键词检索能找到包含特定词汇的memory
  - 语义检索能找到概念相关但词汇不同的memory
  - 时间范围检索能正确过滤时间戳
  - 混合检索的排序合理（相关性高的排在前面）

### Phase 4: 清理和验证
- [ ] 移除所有MEMORY.md/HISTORY.md相关代码
- [ ] 移除MemoryManager中的Markdown读写逻辑
- [ ] 更新配置文件和文档
- [ ] 端到端测试：
  - 新会话从零开始，consolidation正常工作
  - 跨会话检索能找到历史memory
  - 性能测试：检索延迟 < 100ms (P95)
  - 索引同步测试：UPSP更新后索引立即可查
- [ ] 验证：codebase中不再有MEMORY.md/HISTORY.md的引用

## Assumptions Exposed & Resolved

| Assumption | Challenge | Resolution |
|------------|-----------|------------|
| UPSP可以作为交换格式和内部格式 | 是否需要双格式共存？ | 选择混合模式，但最终决定完全替代 |
| UPSP可以在多个数据流节点介入 | 具体在哪个节点集成？ | Consolidation输出侧，转换memory_update |
| UPSP可以补充现有存储 | 替代还是补充MEMORY.md？ | 完全替代，立即废弃Markdown |
| 跨会话检索可以简单实现 | 需要什么检索能力？ | 混合检索（关键词+语义+时间） |
| upsp-rs提供完整功能 | 是否包含索引和查询？ | 仅序列化格式，索引由agent-diva构建 |
| HISTORY.md可以保留 | 是否需要单独的历史日志？ | 废弃，UPSP已具备相应功能 |

## Technical Context

### Current Architecture (Before UPSP)
```
agent-diva-core/
├── src/memory/
│   ├── manager.rs      # MemoryManager: 读写MEMORY.md/HISTORY.md
│   └── storage.rs      # Memory/DailyNote数据结构
├── src/session/
│   ├── manager.rs      # SessionManager: JSONL会话持久化
│   └── store.rs        # Session/ChatMessage数据结构

agent-diva-agent/
├── src/context.rs      # ContextBuilder: 全量注入MEMORY.md到system prompt
└── src/consolidation.rs # Consolidation: messages → LLM → memory_update → MEMORY.md
```

**Current Data Flow:**
1. SessionManager加载JSONL会话
2. 当unconsolidated messages ≥ 100时触发consolidation
3. Consolidation LLM接收旧messages + 现有MEMORY.md
4. LLM输出memory_update和history_entry
5. MemoryManager写入MEMORY.md和追加HISTORY.md
6. ContextBuilder读取完整MEMORY.md注入到每个LLM调用

**Limitations:**
- 全量注入MEMORY.md，无查询机制
- 无结构化索引
- 无语义检索
- Markdown格式不利于程序化处理

### Target Architecture (After UPSP)
```
agent-diva-core/
├── src/memory/
│   ├── upsp_converter.rs   # NEW: memory_update → UPSP转换
│   ├── upsp_storage.rs     # NEW: UPSP格式读写
│   ├── index_manager.rs    # NEW: 管理SQLite和Vector索引
│   └── retrieval.rs        # NEW: 混合检索逻辑
├── src/session/
│   └── (unchanged)         # JSONL会话持久化保持不变

agent-diva-agent/
├── src/context.rs          # MODIFIED: 调用retrieval获取相关片段
└── src/consolidation.rs    # MODIFIED: 输出后调用upsp_converter

Dependencies:
├── upsp-rs                 # UPSP序列化/反序列化
├── sqlx (SQLite)           # 关键词和时间索引
├── qdrant-client / faiss   # 向量存储
└── fastembed / openai      # Embedding模型
```

**New Data Flow:**
1. SessionManager加载JSONL会话（不变）
2. Consolidation触发（不变）
3. Consolidation LLM输出memory_update（不变）
4. **NEW:** UPSP Converter将memory_update转换为UPSP格式
5. **NEW:** UPSP Storage保存UPSP文件
6. **NEW:** Index Manager监听变化，更新SQLite和Vector索引
7. **NEW:** ContextBuilder调用Retrieval Layer获取相关memory片段
8. **NEW:** Retrieval Layer执行混合检索（关键词+语义+时间）
9. 相关片段注入到system prompt

### Key Files to Modify
- `agent-diva-core/src/memory/manager.rs` - 重构为upsp_storage.rs
- `agent-diva-agent/src/context.rs` - 移除全量注入，调用retrieval
- `agent-diva-agent/src/consolidation.rs` - 增加UPSP转换步骤

### Key Files to Create
- `agent-diva-core/src/memory/upsp_converter.rs`
- `agent-diva-core/src/memory/upsp_storage.rs`
- `agent-diva-core/src/memory/index_manager.rs`
- `agent-diva-core/src/memory/retrieval.rs`
- `agent-diva-core/src/memory/vector_store.rs`

## Ontology (Key Entities)

| Entity | Type | Fields | Relationships |
|--------|------|--------|---------------|
| UPSP | external standard | serialization format, protocol specification | converts Memory Update, replaces Markdown Files |
| Memory System | core domain | storage mechanism, retrieval logic, consolidation pipeline | contains all memory components |
| JSONL Sessions | supporting | short-term storage, message history | feeds into Consolidation Pipeline |
| Markdown Files | deprecated | MEMORY.md, HISTORY.md (to be removed) | replaced by UPSP Storage |
| Consolidation Pipeline | core domain | trigger threshold (100 messages), LLM call | produces Memory Update |
| Memory Update | supporting | memory_update content, history_entry content | input to UPSP conversion |
| UPSP Storage | core domain | UPSP format files, storage location | replaces Markdown Files completely |
| ContextBuilder | core domain | system prompt building, memory injection | reads from Retrieval Layer |
| MemoryManager | core domain | load/save operations (refactored to UPSP) | manages UPSP Storage |
| Retrieval Layer | core domain | hybrid retrieval logic, ranking | queries Index Manager |
| Keyword Index | supporting | inverted index, SQLite | part of Index Manager |
| Vector Index | supporting | embedding vectors, similarity search | part of Index Manager |
| Time Index | supporting | timestamp-based filtering | part of Index Manager |
| Embedding Model | external | vector generation | used by Vector Index |
| upsp-rs | external library | serialization/deserialization only | used by UPSP Converter |
| Index Manager | core domain | index synchronization, consistency | manages all indexes |
| Vector Database | supporting | qdrant/milvus/faiss | stores semantic vectors |
| SQLite Index | supporting | keyword and time queries | structured query engine |
| 长期记忆场景 | use case | UPSP-based storage | replaced Markdown-based storage |
| 跨会话检索场景 | use case | hybrid retrieval strategy | enabled by Index Manager |

## Ontology Convergence

| Round | Entity Count | New | Changed | Stable | Stability Ratio |
|-------|-------------|-----|---------|--------|----------------|
| 1 | 6 | 6 | - | - | - |
| 2 | 9 | 3 | 0 | 6 | 67% |
| 3 | 11 | 2 | 2 | 7 | 82% |
| 4 | 16 | 5 | 1 | 10 | 69% |
| 5 | 20 | 4 | 1 | 15 | 80% |
| 6 | 20 | 0 | 0 | 20 | 100% |

**Convergence Analysis:** Ontology stabilized at round 6 with 100% stability. Core concepts (UPSP, Memory System, Consolidation Pipeline) remained stable from round 2 onwards. Implementation details (indexes, retrieval layer) were added in rounds 4-5 but did not change the core architecture. This indicates a solid architectural foundation with clear implementation path.

## Interview Transcript

<details>
<summary>Full Q&A (6 rounds)</summary>

### Round 1
**Q:** UPSP（Universal Prompt Serialization Protocol）的核心目标是什么？是要让agent-diva的memory系统能够序列化/反序列化为UPSP格式以便跨系统交换，还是要用UPSP作为内部存储格式替代现有的JSONL+Markdown，或者是两者都要？

**A:** 混合模式：内部存储保持现状，但在特定场景（如长期记忆、跨会话检索）使用UPSP格式，形成双格式共存

**Ambiguity:** 76%
- Goal: 0.30 (UPSP定位为混合模式，但具体使用场景的目标不明确)
- Constraints: 0.20 ("特定场景"边界模糊，不知道何时使用UPSP vs 现有格式)
- Criteria: 0.10 (完全没有验证标准)
- Context: 0.40 (了解现有架构，知道UPSP是增强而非替代)

### Round 2
**Q:** 你提到"长期记忆"和"跨会话检索"是UPSP的使用场景。在agent-diva现有架构中，consolidation机制已经处理长期记忆（messages → MEMORY.md），ContextBuilder负责注入memory到prompt。UPSP应该在哪个具体的数据流节点介入？

**A:** Consolidation输出侧：Consolidation LLM输出memory_update后，将其转为UPSP格式存储，替代或补充现有的MEMORY.md

**Ambiguity:** 59%
- Goal: 0.50 (UPSP作为consolidation输出的格式转换层，但最终目标仍需细化)
- Constraints: 0.50 (明确了介入点，但"替代或补充"MEMORY.md的策略未定)
- Criteria: 0.15 (略有提升，但仍缺乏具体验证标准)
- Context: 0.50 (清楚UPSP在consolidation pipeline中的位置)

### Round 3
**Q:** UPSP格式应该"替代"还是"补充"现有的MEMORY.md？

**A:** 完全替代：废弃MEMORY.md，所有memory以UPSP格式存储。用户明确表示"现状阶段，就立刻丢弃MEMORY.MD，因为UPSP已经有类似实现"

**Ambiguity:** 39%
- Goal: 0.70 (UPSP成为核心存储层，目标清晰)
- Constraints: 0.80 (存储策略明确：完全替代MEMORY.md)
- Criteria: 0.30 (可验证MEMORY.md废弃，但跨会话检索实现方式未明确)
- Context: 0.60 (清楚改造范围：ContextBuilder、MemoryManager)

### Round 4
**Q:** 跨会话检索应该如何工作？当前agent-diva的限制：ContextBuilder将完整MEMORY.md注入每个prompt，没有查询机制。

**A:** 混合检索：结合多种检索策略（关键词+语义+时间）

**Ambiguity:** 25%
- Goal: 0.80 (跨会话检索目标清晰：多维度混合检索)
- Constraints: 0.85 (需要embedding模型、多维索引，但UPSP格式能力边界未明确)
- Criteria: 0.60 (可验证检索质量：关键词/语义/时间维度)
- Context: 0.70 (需要新增检索层，架构清晰)

### Round 5
**Q:** upsp-rs（已完成的UPSP Rust实现）是否原生提供索引和查询能力？

**A:** 仅序列化格式：UPSP只是数据格式规范，agent-diva需要自己构建索引层（如SQLite + 向量数据库）

**Ambiguity:** 19%
- Goal: 0.85 (需要构建完整检索基础设施，目标明确)
- Constraints: 0.90 (upsp-rs只做序列化，索引由agent-diva负责)
- Criteria: 0.70 (可验证索引构建、查询性能、同步一致性)
- Context: 0.75 (需要新增索引管理组件，架构清晰)

### Round 6
**Q:** HISTORY.md应该如何处理？

**A:** 同样废弃：HISTORY.md也用UPSP格式替代，因为UPSP已经具有了相对应的功能

**Ambiguity:** 14%
- Goal: 0.90 (完全清晰：UPSP替代所有Markdown存储)
- Constraints: 0.95 (边界明确：废弃MEMORY.md和HISTORY.md)
- Criteria: 0.75 (可验证：所有Markdown功能迁移到UPSP)
- Context: 0.80 (改造范围完全清晰)

</details>

## Implementation Phases

### Phase 0: Preparation (Week 1)
- Review upsp-rs API documentation
- Select vector database (qdrant vs milvus vs faiss)
- Select embedding model (local vs API)
- Design UPSP storage schema
- Design index schema (SQLite tables + vector dimensions)

### Phase 1: UPSP Integration (Week 2-3)
- Integrate upsp-rs into agent-diva-core
- Implement UPSP Converter
- Modify consolidation.rs to call converter
- Implement UPSP Storage (replace MemoryManager Markdown logic)
- Unit tests for conversion and storage

### Phase 2: Index Layer (Week 4-5)
- Implement SQLite Index (keyword + time)
- Integrate vector database
- Integrate embedding model
- Implement Index Manager (sync logic)
- Unit tests for indexing

### Phase 3: Retrieval Layer (Week 6-7)
- Implement Retrieval Layer (hybrid search)
- Modify ContextBuilder to use retrieval
- Integration tests for end-to-end retrieval
- Performance benchmarking (target: <100ms P95)

### Phase 4: Cleanup & Migration (Week 8)
- Remove all MEMORY.md/HISTORY.md code
- Migration script for existing Markdown files → UPSP
- Update documentation
- End-to-end testing
- Performance validation

## Success Metrics

### Functional Metrics
- ✅ Consolidation produces UPSP files instead of Markdown
- ✅ All memory operations use UPSP format
- ✅ Keyword retrieval precision > 90%
- ✅ Semantic retrieval finds conceptually related memories
- ✅ Time range filtering works correctly
- ✅ Hybrid retrieval ranking is reasonable (manual evaluation)

### Performance Metrics
- ✅ Retrieval latency < 100ms (P95)
- ✅ Index update latency < 500ms after UPSP write
- ✅ Memory footprint increase < 50% (due to indexes)
- ✅ No MEMORY.md/HISTORY.md references in codebase

### Quality Metrics
- ✅ Unit test coverage > 80% for new components
- ✅ Integration tests cover all retrieval scenarios
- ✅ Zero data loss during migration
- ✅ Index consistency: UPSP and indexes always in sync

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| upsp-rs API不稳定 | High | 在Phase 0详细review API，必要时贡献PR |
| 向量数据库性能不足 | Medium | 在Phase 0进行benchmark，选择最优方案 |
| Embedding模型延迟高 | Medium | 使用本地模型或批量embedding |
| 索引同步失败 | High | 实现事务性更新，失败时回滚 |
| 迁移数据丢失 | Critical | 迁移前备份，迁移后验证 |
| 检索质量不佳 | Medium | 实现可调参数，支持A/B测试 |

## Open Questions (Resolved)

All questions resolved during interview. No open questions remaining.

## Next Steps

**Recommended execution path:** Ralplan → Autopilot (3-stage pipeline)

1. **Stage 1 (Complete):** Deep Interview - Requirements clarified, ambiguity 14%
2. **Stage 2 (Next):** Ralplan with consensus - Planner/Architect/Critic refine implementation plan
3. **Stage 3 (Final):** Autopilot execution - Parallel implementation with QA cycling

This spec is ready for consensus refinement via `omc-plan --consensus --direct`.
