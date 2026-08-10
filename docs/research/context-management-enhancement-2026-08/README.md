# 上下文管理增强（2026-08）

HARNESS-GAP-RESEARCH 在 **Context / Prompt Cache / 预算 / 工具结果 / 按需工具** 方向上的定向调研与架构决策。

## 阅读顺序

1. **[c0-baseline-and-architecture-decisions.md](./c0-baseline-and-architecture-decisions.md)**（主报告）
   - Agent-Diva 每轮上下文数据流基线与 cache-break 清单
   - Codex / Claude Code+OpenHarness / GenericAgent 定向对照
   - 冻结 ADR-CTX-0..5 与 C1–C5 实施切片

## 主线

```text
C0 测量（本文档）→ C1 稳定前缀 → C2 类型化预算 → C3 工具结果引用 → C4 按需工具/Recall → C5 验收
```

## 边界

- **只做调研与 ADR，不写运行时代码**（本目录交付）。
- **不重做** BML/Laputa 权威；遵守 `docs/architecture/memory-write-paths-contract.md`。
- **不复活** 已关闭的 OpenHarness dry-run / ohmo 提案。
- Plan Mode 硬状态机、Subagent Worktree 等其它 HARNESS-GAP 方向 **分轨**，不在本专题实施范围。

## 样本

| 目标 | 样本 |
|------|------|
| 长会话压缩与状态延续 | `.workspace/codex` |
| Prompt Cache、工具延迟挂载 | `.workspace/claude-code`、`.workspace/OpenHarness` |
| Memory 与上下文分层 | `.workspace/GenericAgent` |
| 落点 | 本仓库 `prepare_runtime_context` / `ContextBuilder` / `ToolAssembly` / BML |
