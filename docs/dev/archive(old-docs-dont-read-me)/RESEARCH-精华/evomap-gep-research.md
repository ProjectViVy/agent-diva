# EvoMap / GEP 调研精华（压缩版）

> 原始：`MOREDIVA-EvoMap-调研与方向.md`（112 行）
> 核心结论：EvoMap 已闭源，只用 Apache-2.0 MCP 包；自研 skill 进化是护城河

---

## 1. 背景

- **EvoMap/evolver**：8180+ stars，基于 GEP（Genome Evolution Protocol）
- **状态**：核心 CLI 已闭源（commit 247→3），MCP 桥接包 `gep-mcp-server` v1.7.0 仍为 Apache-2.0
- **指控**：EvoMap 公开指控 Hermes "未引用即借鉴"（Gene→Skill、Capsule→Validated Skill 等 1:1 改名）

## 2. 接入策略

| 层级 | 方案 | 风险 |
|------|------|------|
| 浅层 | MCP 桥 read-only | 低，合规 |
| 中层 | Hub 注册（opt-in） | 中，需 node_secret 持久化 |
| 深层 | 原生 gep-a2a 协议 | 高，EvoMap 闭源后协议可能变动 |

**建议**：先浅层（MCP 桥），Hub 注册等大湿拍板。

## 3. 自研 vs GEP 路线对比

| 维度 | EvoMap GEP | 自研（周报提取） |
|------|-----------|----------------|
| 核心机制 | LLM+GEPA 优化 SKILL.md | 从 session 日志提取 skill |
| 优势 | 社区生态（8180+ users） | 不依赖外部、不付 Credits |
| 劣势 | 闭源风险、商业关系敏感 | 需要自建 pipeline |
| 命名 | Gene/Capsule/EvolutionEvent | Skill/MemoryEntry/HistoryEntry（避嫌） |

## 4. 待决策

- [ ] Hub 注册是否做？（opt-in）
- [ ] "周报"指 LLM 自动生成还是人工撰写？
- [ ] 提取出的 skill 是否 publish 到 EvoMap Hub？

## 5. 原始文档

- `morediva-root/old-docs/MOREDIVA-EvoMap-调研与方向.md`
