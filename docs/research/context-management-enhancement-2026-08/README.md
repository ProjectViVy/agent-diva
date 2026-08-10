# 上下文管理增强（2026-08）

HARNESS-GAP-RESEARCH 在 **Context / Prompt Cache / 预算 / 工具结果 / 按需工具** 方向上的定向调研与架构决策。

## 阅读顺序

1. **[claude-code-prompt-cache-alignment.md](./claude-code-prompt-cache-alignment.md)**（**优先** — Cache / C1 施工图）
   - 仅深读 Claude Code：section 缓存、tool 前缀、break detection、sticky latch
   - Diva bust 清单 B1–B9 与 P0-1..P0-5 实施规格、测试 T1–T8
2. **[c0-baseline-and-architecture-decisions.md](./c0-baseline-and-architecture-decisions.md)**（总论）
   - Agent-Diva 每轮上下文数据流基线
   - Codex / Claude Code / GenericAgent 定向对照
   - 冻结 ADR-CTX-0..5 与 C1–C5 切片

## 主线

```text
C0 测量 → C1 稳定前缀/Prompt Cache（见 claude-code-prompt-cache-alignment）
  → C2 类型化预算 → C3 工具结果引用 → C4 按需工具/Recall → C5 验收
```

## 实施前冻结决策（2026-08-10 修订）

以下决策是 C1–C5 的施工门，不是可选建议；详细契约见总论 §6.6，C1 细节见
Prompt Cache 专章 §4.4、§5：

1. 动态块保持在逻辑 post-prefix 区，但其 provider wire role 由 capability-aware
   serializer 决定；默认不得假设 provider 支持对话中段 `system`。
2. C1 必须复用 C1-0 已落地的 `PromptSection` / `SectionStability` / 固定顺序；
   `prefix_hash` 与观测在 P0-4 接入，禁止退回字符串搬运。
3. Stable/SessionStable 采用「会话快照优先、显式事件失效」，禁止每轮轮询文件系统。
4. Tool schema 稳定排序与 provider `cache_control` 锚点必须分为两个原子提交。
5. C3 在写 artifact store 前必须先满足隔离、retention、容量、脱敏、不可伪造 key、
   重启/缺失语义和 tool-call 配对安全契约。
6. C4 mount 在同一 turn 的下一次 provider call 生效；发现集合绑定 session，且每次
   重建仍须经过 mask/plan policy。
7. Cache 观测必须记录 provider/model/policy 与连续趋势；预期 break、策略变化和
   无结构变化的异常 miss 分级处理，禁止仅凭一次 `cache_read` 下降告警。

**当前进度：** C1-0 已完成最小类型与 characterization tests；生产序列化尚未迁移。

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
