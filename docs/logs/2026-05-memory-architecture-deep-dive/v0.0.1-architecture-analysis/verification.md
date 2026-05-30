# 验证记录：记忆架构深层研究

## 版本信息
- 版本号: v0.0.1-architecture-analysis
- 验证日期: 2026-05-28

---

## 验证项目

### 1. GenericAgent L0-L4 存储形态确认

| 检查项 | 结果 | 证据 |
|---|---|---|
| L0 是否为文件？ | ✅ 否，是 SOP 中的四条公理 | `.workspace/GenericAgent/.qoder/repowiki/zh/content/核心架构设计/分层记忆系统/L0元规则层.md` |
| L1 是否为纯文本 ≤30行？ | ✅ 是 | `.workspace/GenericAgent/.qoder/repowiki/zh/content/核心架构设计/分层记忆系统/L1索引层.md` 确认硬约束 |
| L2 是否为纯文本？ | ✅ 是，`global_mem.txt` 按 section 组织 | wiki 分层记忆系统.md L2 事实库层 |
| L3 是否为文件系统？ | ✅ 是，`memory/*.md` + `memory/*.py` | wiki L3 记录库层 |
| L4 是否有压缩脚本？ | ✅ 是，`compress_session.py` | wiki L4 会话归档层 |

### 2. Mentle 数据库结构确认

| 检查项 | 结果 | 证据 |
|---|---|---|
| SQLite 表结构 | ✅ 6 张表 | `.workspace/memtle/src/schema.rs:8-101` |
| drawers 表有 wing/room 字段 | ✅ 有索引 | `schema.rs:22-24` |
| BM25 倒排索引 | ✅ `drawer_words` 表 | `schema.rs:30-38` |
| 知识图谱支持 | ✅ `entities` + `triples` 表 | `schema.rs:40-65` |
| 跨 wing 隧道 | ✅ `explicit_tunnels` 表 | `schema.rs:86-101` |
| AAAK 压缩 | ✅ `compressed` 表 | `schema.rs:67-75` |

### 3. Mentle 层级实现确认

| 检查项 | 结果 | 证据 |
|---|---|---|
| L0 identity.txt | ✅ 独立函数 | `.workspace/memtle/src/palace/layers.rs:29-51` |
| L1 essential story | ✅ ≤15 drawers + 3200 chars cap | `layers.rs:12-18` |
| L2 按 wing/room 过滤 | ✅ 100 results max | `layers.rs:19-21` |
| L3 BM25 深度搜索 | ✅ 邻居扩展 | `layers.rs:365-456` |
| MemoryStack 聚合器 | ✅ recall/browse/search/status | `.workspace/memtle/src/palace/stack.rs` |

### 4. 正交维度论证

| 检查项 | 结果 | 证据 |
|---|---|---|
| GenericAgent 的层是类型轴 | ✅ L0=规则 / L1=索引 / L2=事实 / L3=SOP / L4=归档 | wiki 全部层级文档 |
| Mentle 的 wing 是领域轴 | ✅ 项目级分区 | `schema.rs` drawers.wing 字段 + `room_detect.rs` 70+ 映射 |
| 二者不重叠 | ✅ 一个 wing 含多种层类型 | `graph.rs` 中 BFS 遍历跨 wing 连接 |

### 5. 上一轮调研方案修正确认

| 检查项 | 结果 | 证据 |
|---|---|---|
| v0.0.1-research-synthesis 方案中 mentle 定位 | 深层事实与证据层（Phase 1） | `docs/logs/2026-05-genericagent-mentle-user-controlled-learning/v0.0.1-research-synthesis/summary.md:48` |
| 本轮修正 | mentle 降为 Phase 2 存储引擎 | `summary.md` 架构判断章节 |
| 修正理由 | L0-L4 是文件纪律体系，不依赖存储引擎 | 发现 1-2 |

### 6. 公理体系原始来源验证

| 公理 | 原始位置 | 验证 |
|---|---|---|
| 行动验证优先 | `memory_management_sop.md` | ✅ |
| 神圣不可删改 | `memory_management_sop.md` | ✅ |
| 禁止易变状态 | `memory_management_sop.md` | ✅ |
| 最小充分指针 | `memory_management_sop.md` | ✅ |

---

## 验证结论

全部 6 大类 24 项检查通过。架构判断基于实际源码和文档，非推测。
