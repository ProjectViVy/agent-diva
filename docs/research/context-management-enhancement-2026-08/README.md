# 上下文管理增强（2026-08）

> 当前状态（2026-08-13）：C1–C5 生产实现与验证已完成；本目录保留施工决策、边界和
> 证据索引。C5 文档中的“实施前”表述只描述当时的施工阶段，不能覆盖最新日志结论。

HARNESS-GAP-RESEARCH 在 **Context / Prompt Cache / 预算 / 工具结果 / 按需工具** 方向上的定向调研与架构决策。

## 阅读顺序

1. **[c5-lightweight-context-convergence.md](./c5-lightweight-context-convergence.md)**（**当前运行时合同** — C5 轻量收敛与 clean break）
   - 三段上下文、单一检查点、两级轻量压缩
   - C1–C4 新功能禁兼容、垃圾状态和双路径删除门禁
2. **[claude-code-prompt-cache-alignment.md](./claude-code-prompt-cache-alignment.md)**（Cache / C1 历史施工图）
   - 仅深读 Claude Code：section 缓存、tool 前缀、break detection、sticky latch
   - Diva bust 清单 B1–B9 与 P0-1..P0-5 实施规格、测试 T1–T8
3. **[c0-baseline-and-architecture-decisions.md](./c0-baseline-and-architecture-decisions.md)**（总论）
   - Agent-Diva 每轮上下文数据流基线
   - Codex / Claude Code / GenericAgent 定向对照
   - 冻结 ADR-CTX-0..5 与 C1–C5 切片

## 主线

```text
C0 测量 → C1 稳定前缀 → C2 类型化预算 → C3 工具结果引用
  → C4 按需工具/Recall → C5 轻量上下文收敛与长任务验收
```

## 已冻结并已验证的运行时决策（2026-08-10/11）

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

**当前进度：** C1-0、C1a/P0-1、C1b/P0-3、C1c/P0-2、C1d/P0-4..5、
CTX-C2、CTX-C3、CTX-C4 与 C5a–C5e 均已完成。stable system
已移除 Current Time/session/WM/Recall/Plan 等动态内容；工具定义固定为 CORE
字典序连续前缀 + MCP/custom DEFERRED 字典序后缀；四个稳定 section 现在按 session
快照缓存，mask/L1 只刷新目标 section，并携带版本化 break reason；生产请求记录
system/tools/per-tool hash 与 cache usage，cache-control 只锚定第一条 stable system 和
CORE 工具段末。C2 已将稳定规则、CORE/DEFERRED schema、L1、WM、Recall、
Compaction、History、ToolResult 与当前回合纳入分层实际计量，并强制生成
AssemblyReport。C3/C4 已完成 artifact 引用、microcompact、按需工具和 Recall 矩阵；
下一 Context 主线改为 C5 轻量收敛，按当前施工权威删除重复状态和旧压缩路径。

C1c 采用聚焦边界：同一 AgentLoop/provider 实例内的 memory 写入通过 startup revision
触发 L1 热刷新；Manager 外部治理 apply、Skills 管理入口到 runtime invalidation 的通知
接线已登记在 `TODOLIST.md`，不以每轮文件系统轮询替代。

## 边界

- 本目录仍是调研与 ADR 权威入口；运行时代码按 C1a–C1d 切片在对应 crate 落地。
- **不重做** BML/Laputa 权威；遵守当前 [Cognitive Workspace 边界](../../architecture/current/cognitive-workspace-boundaries-2026-08.md)；旧 Memory 写路径合同仅作归档证据。
- **不复活** 已关闭的 OpenHarness dry-run / ohmo 提案。
- Plan Mode 硬状态机、Subagent Worktree 等其它 HARNESS-GAP 方向 **分轨**，不在本专题实施范围。

## 样本

| 目标 | 样本 |
|------|------|
| 长会话压缩与状态延续 | `.workspace/codex` |
| Prompt Cache、工具延迟挂载 | `.workspace/claude-code`、`.workspace/OpenHarness` |
| Memory 与上下文分层 | `.workspace/GenericAgent` |
| 落点 | 本仓库 `prepare_runtime_context` / `ContextBuilder` / `ToolAssembly` / BML |
