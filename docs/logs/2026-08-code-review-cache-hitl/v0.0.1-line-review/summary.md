# Summary — 缓存/工具调用链 + HITL 逐行审查

**基线：** `b4d2a84b` on `agent-diva-pro`（ahead origin 61）  
**日期：** 2026-08-12  
**方法：** 分层逐行（L1 热路径全文 + L2 调用图 + L3 删除证明 + L4 覆盖矩阵）  
**范围：** 见 `findings.md` / plan

## 结论（一句话）

- **Track A（CTX C1–C5）已在 `agent-diva-pro` HEAD 上落地**，工具结果引用化、deferred 自动激活、canonical checkpoint、final-wire 观测主路径整体可用，自动化测试较厚；前端对 artifact/compact 几乎无专门 UX。
- **Track B「M3 HITL 三模式增强」与「2026-07 统一 HITL interaction store」并不在 HEAD**：分别只在 `feat/m3-hitl-closure` 与 `refactor/deep-governance`。HEAD 上仍是**半残审批三模式 + 统一 ApprovalCenter + ask_user 已接 GUI**。
- **最高优先级问题是交付状态与代码事实不一致**（日志/TODOLIST 写“已完成”，当前分支却无对应 commit），其次是 **Guardian 三模式在生产 shell 未接线**、**legacy 审批流仍启动但前端不消费**。

## 审查覆盖

| 主题 | HEAD 状态 | L1 精读 |
|------|-----------|---------|
| C1–C2 cache / budget | 在 HEAD | cache_observe、final_wire、openai/anthropic 塑形 |
| C3 artifact + microcompact | 在 HEAD | tool_artifact、tool_results、tool_step、iteration |
| C4/C5e deferred auto-activate | 在 HEAD | registry、tool_discovery、agent_loop 测试 |
| C5b–d checkpoint/recovery | 在 HEAD | compaction_exec、session 路径点检 |
| ask_user clarify | 在 HEAD | core/tools/manager/GUI/CLI |
| M3 S1–S5 三模式 | **仅 feat/m3-hitl-closure** | HEAD 对照 + 侧分支 diff |
| 2026-07 durable HITL store | **仅 refactor/deep-governance** | 不在 HEAD；HEAD 用 ApprovalCenter 另一路径 |

## 发现数量（摘要）

| 级 | 数量 | 代表 |
|----|------|------|
| S0 | 0 | — |
| S1 | 3 | M3 未合入却文档已 close；生产 shell 无 mode-driven Guardian；三模式 review 仍合并分支 |
| S2 | 5 | 非 cache 提供商 final-wire core 哈希空；legacy 审批 SSE 空转；GUI 模式不持久；artifact GUI 不解析；文档 C4 过期 |
| S3 / Gap | 若干 | 覆盖矩阵见 `coverage-matrix.md` |

## 交付物

- `findings.md` — 按 severity 排序
- `coverage-matrix.md` — 功能 × 证明层级
- `verification.md` — 证据与命令
- `acceptance.md` / `release.md`

## 建议下一步

1. **立刻纠正过程真相：** TODOLIST / LOCK handoff 中 M3「已完成」改为「侧分支未合入」，避免并行会话基于错误基线。
2. **合并决策：** 评估 `feat/m3-hitl-closure` rebase 到 `agent-diva-pro`（或显式废弃并重做）。
3. **HEAD 热修候选：** 去掉或消费 `start_command_approval_stream`；permissionMode localStorage 持久化（可不依赖整包 M3）。
4. **Track A 前端：** tool row 解析 `ToolResultRef` 展示 preview + 引导 `read_tool_result`（Gap，非阻断）。
