# 记忆架构深层研究：GenericAgent L0-L4 × Mentle 可行性与分阶段策略

## 版本信息
- 版本号: v0.0.1-architecture-analysis
- 调研日期: 2026-05-28
- 范围: `.workspace/GenericAgent`、`.workspace/memtle`、`docs/dev/genericagent/`、`docs/logs/` 全部历史调研记录
- 约束: 只读调研，不修改业务代码

---

## 核心结论

**Phase 1 不碰 mentle。纯文件 + 公理 + 分类决策树即可达到 80% 效果。mentle 是 Phase 2 的存储引擎升级路径。**

这一结论纠正了上一轮调研（v0.0.1-research-synthesis）中将 mentle 作为第一阶段核心组件的方案。

---

## 研究背景

上一轮调研（2026-05-27）提出了"MEMORY.md + mentle + Learning Index"三件套方案，并将 mentle 定位为"深层事实与证据层"。本轮重新审视了 GenericAgent 源码后发现：

1. GenericAgent 的 L0-L4 不是技术架构，是**文件纪律体系**
2. Mentle 的 wing/room/drawer 与 GenericAgent 的 layer 是**正交维度**
3. 整个记忆系统的 load-bearing 组件是**四条公理 + 分类决策树**，不是存储引擎

---

## 关键发现

### 发现 1：GenericAgent L0-L4 的真实形态

| 层 | 存储形式 | 本质 |
|---|---|---|
| L0 | 不是文件，是写在 SOP 里的四条公理 | 行为约束（action-verified / 不可删改 / 禁易变状态 / 最小指针） |
| L1 | `global_mem_insight.txt`（≤30行，<1k tokens） | 纯文本索引，全量注入 system prompt |
| L2 | `global_mem.txt`（按 `## [SECTION]` 组织） | 纯文本事实库 |
| L3 | `memory/*.md` + `memory/*.py` | 文件系统的 SOP 和脚本 |
| L4 | `L4_raw_sessions/*.txt` → `compress_session.py` → `.zip` | Python 压缩脚本 + 文件归档 |

**关键事实：没有数据库，没有搜索引擎。整个检索就是 LLM 读 L1 → 定向 `file_read` L2/L3。**

来源：`.workspace/GenericAgent/.qoder/repowiki/zh/content/核心架构设计/分层记忆系统/` 全部 wiki 页面、`memory_management_sop.md`。

### 发现 2：四条公理才是真正 load-bearing 的组件

```
公理 1: 行动验证优先 — 没经过工具调用验证的，不写入
公理 2: 神圣不可删改 — 重构不丢已验证配置
公理 3: 禁止易变状态 — 不存 PID、时间戳、session ID
公理 4: 最小充分指针 — L1 只存"往哪找"，不存内容本身
```

加上 L1 硬约束（≤30行）和分类决策树（事实→L2 / 操作规律→L1 RULES / 任务技巧→L3 / 其余→丢弃），整个系统靠**纪律**运转，不靠技术栈。

来源：`.workspace/GenericAgent/.qoder/repowiki/zh/content/核心架构设计/分层记忆系统/L0元规则层.md`、`L1索引层.md`、`记忆分类决策树.md`。

### 发现 3：Mentle 的 wing/room/drawer 与 GenericAgent 的 layer 是正交维度

**GenericAgent 的"层"是记忆类型分层（垂直轴）：**
- L0 = 元规则 / L1 = 索引 / L2 = 事实 / L3 = SOP / L4 = 归档

**Mentle 的"wing"是项目/上下文分区（水平轴）：**
- wing = 项目/领域（如 `agent-diva`、`memtle`）
- room = wing 内功能区（来自文件夹映射，70+ 自动检测规则）
- drawer = 最小存储单元（~800 字文本块 + 倒排索引词）

**这两个维度不应直接映射，应该正交使用：**
- 一个 wing 里会有多种"层"类型的内容
- 一种"层"可以跨多个 wing 存在

正确的对应：用 wing 做领域分区，用 room 做类型分区，用 drawer 内容的 `extract_mode` 做层分类。

来源：`.workspace/memtle/src/palace/layers.rs`、`graph.rs`、`schema.rs`、`stack.rs`。

### 发现 4：Mentle 的实际能力边界

| 能力 | agent-diva 现状 | mentle 提供 |
|---|---|---|
| 结构化记忆检索 | MEMORY.md 全量注入 | BM25 精确检索 + wing/room 过滤 |
| 跨项目知识图谱 | 无 | entities + triples + tunnels |
| 渐进式记忆累积 | 每次 consolidation 重写 | append-only drawer + 倒排索引自动维护 |
| Obsidian 集成 | 无 | `export` 命令导出 vault 格式 |

**但这些能力在 Phase 1 不是瓶颈。** agent-diva 当前的记忆体量远未达到需要 BM25 检索或知识图谱的规模。

来源：`.workspace/memtle/README.md`、`.workspace/memtle/src/lib.rs`。

### 发现 5：AAAK 压缩在运行时不需要

AAAK（entity codes + emotion markers + pipe-separated fields）的价值在 Obsidian 侧导出时生成紧凑的 daily note / topic note。在 agent-diva 内部运行时，drawer 本身就是最小存储单元，BM25 检索已经足够。

来源：`.workspace/memtle/src/dialect/`、`.workspace/memtle/src/tools/protocol.rs`。

---

## 架构判断

### 当前方案修正

上一轮方案（v0.0.1-research-synthesis）建议：
> mentle: 深层事实与证据（房间/抽屉/KG）

**修正为：**
> mentle 在 Phase 1 不引入。Phase 1 纯文件 + 公理。mentle 留到 Phase 2 作为存储引擎升级路径。

### 分层架构视图

```
Phase 1（当前）                     Phase 2（L3 膨胀后）
─────────────────                   ──────────────────
L0 公理（写在 SOP/AGENTS.md）       L0 公理（不变）
L1 索引（≤30行纯文本）              L1 索引（增加 mentle room 导航）
L2 事实（memory/facts.md）          L2 事实（可选迁入 mentle drawers）
L3 SOP（memory/sop/*.md）           L3 SOP（mentle BM25 检索替代遍历）
                                    L4 归档（mentle compressed 表 + export）
```

### mentle 作为存储引擎的定位

mentle 不参与核心记忆循环。它是"当文件系统不够用时的升级路径"：
- 当 L3 SOP 膨胀到 LLM 无法遍历时 → mine 进 SQLite → BM25 检索
- 当需要跨项目知识图谱时 → entities + triples + tunnels
- 当需要 Obsidian 导出时 → `mentle export` 到 vault

---

## Phase 1 实施建议

### 1. 注入 L0 公理
将 GenericAgent 的四条公理适配为 agent-diva 版本，写入 `AGENTS.md` 或独立 SOP 文件。

### 2. 创建 L1 索引
在 `memory/` 下创建 `global_mem_insight.txt`（或等效文件），硬约束 ≤30 行、<1k tokens。启动时全量注入 system prompt。

### 3. 建立 L2 事实库
创建 `memory/facts.md`，按 `## [SECTION]` 组织环境特异性事实（路径、配置、凭证引用）。

### 4. 建立 L3 SOP 目录
创建 `memory/sop/`，按任务归档可复用流程和关键前置条件。

### 5. 建立分类决策树
```
信息进入 → 是环境特异性事实？ → L2
         → 是通用操作规律？   → L1 RULES
         → 是任务级技巧？     → L3 SOP
         → 其余              → 丢弃（LLM 可推理的通用常识不记录）
```

### 6. 在 consolidation 时执行公理验证
- 未经过工具调用验证的 → 不写入
- 易变状态（PID、session ID、时间戳） → 不写入
- 上层只存指针，不存内容

---

## 与 laputa / oh-my-mempalace 的关系

laputa 当前被认为"太复杂"的原因是它在第一阶段就做了状态机 + 投影 + 摘要的完整管线。

GenericAgent 证明：**纯文件 + 公理可以达到 80% 效果。** laputa 的状态机、mentle 的 BM25、Obsidian 的多层索引——都是第二阶段增强，不是第一阶段前提。

mentle + Obsidian 作为未来的外接知识库方案依然成立，但时机在 L3 膨胀到文件系统检索效率不足时。

---

## 参考文献

| 文档 | 路径 |
|---|---|
| GenericAgent 分层记忆系统 wiki | `.workspace/GenericAgent/.qoder/repowiki/zh/content/核心架构设计/分层记忆系统/` |
| GenericAgent L0 元规则 | `.workspace/GenericAgent/.qoder/repowiki/zh/content/核心架构设计/分层记忆系统/L0元规则层.md` |
| GenericAgent L1 索引层 | `.workspace/GenericAgent/.qoder/repowiki/zh/content/核心架构设计/分层记忆系统/L1索引层.md` |
| GenericAgent 记忆分类决策树 | `.workspace/GenericAgent/.qoder/repowiki/zh/content/核心架构设计/分层记忆系统/记忆分类决策树.md` |
| Mentle README | `.workspace/memtle/README.md` |
| Mentle schema | `.workspace/memtle/src/schema.rs` |
| Mentle layers | `.workspace/memtle/src/palace/layers.rs` |
| Mentle graph | `.workspace/memtle/src/palace/graph.rs` |
| 上一轮调研 | `docs/logs/2026-05-genericagent-mentle-user-controlled-learning/v0.0.1-research-synthesis/summary.md` |
| 初始调研 | `docs/logs/genericagent-upgrade-research/v0.0.1-initial-research/summary.md` |
| Plan Mode 调研 | `docs/logs/2026-05-planmode-research/v0.0.1-codex-genericagent-diva/summary.md` |

本版本为调研沉淀文档，不含代码变更。
