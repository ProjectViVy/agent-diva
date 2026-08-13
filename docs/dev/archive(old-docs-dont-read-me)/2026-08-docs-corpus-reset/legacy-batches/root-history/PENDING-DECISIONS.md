# Archive Governance — PENDING DECISIONS

> 生成日期：2026-06-13
> 范围：从 archive 中提取的待拍板事项
> 规则：每个条目必须包含「现状」「选项」「建议」「阻塞点」
>
> **更新**：PD-02 ~ PD-14 已由大湿确认完成，已移至 DONE.md。

---

## 一、架构方向（高优先级）

### PD-01: EvoMap / GEP 原生支持

- **现状**：调研已完成（2026-06-07），EvoMap CLI 已闭源，MCP 桥接包 Apache-2.0 可用
- **选项**：
  - A. 接入 `@evomap/gep-mcp-server` v1.7.0 作为 MCP 桥（只读）
  - B. 内部实现 gep-a2a 协议（深度集成）
  - C. 完全不接入，专注自研 skill 进化
- **建议**：选 A（MCP 桥 read-only），Hub 注册 opt-in
- **阻塞点**：需确认是否注册 Hub（涉及 node_secret 持久化）
- **相关原始文档**：`morediva-root/old-docs/MOREDIVA-EvoMap-调研与方向.md`

---

## 决策记录表

| 编号 | 事项 | 优先级 | 状态 | 阻塞点 |
|------|------|--------|------|--------|
| PD-01 | EvoMap GEP 接入 | 高 | 待拍板 | Hub 注册决策 |
