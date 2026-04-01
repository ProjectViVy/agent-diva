---
story_key: 5-4-orchestration-spi-shannon-bridge
story_id: "5.4"
epic: 5
status: done
generated: "2026-03-31T18:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
---

# Story 5.4：编排适配 SPI 与 Shannon / Agents 路线评估

状态：done

## 故事陈述

作为 **架构维护者**，  
我希望 **用 ADR + 最小端口（trait）描述「外部编排器如何接到 FullSwarm」**，  
以便 **后续可评估 Shannon、openai-agents-python 宿主或独立进程**，且 **不违反 ADR-A（swarm crate 不依赖 meta）**。

## 验收标准

1. **Given** 新 ADR（建议路径：`_bmad-output/planning-artifacts/` 或 `agent-diva-swarm/docs/` —— 实现时单点冻结）  
   **When** 读者跟随文档  
   **Then** 理解：**何时** 调用外部编排、**输入/输出** DTO、**与 `ExecutionTier::FullSwarm` 的边界**、**失败降级**（回退内置序曲或 Light）

2. **And** 附 **≤2 页** 决策备忘：**嵌入式 Python** vs **侧车进程** vs **纯 Rust 扩展** —— **推荐顺序与理由**（不要求本故事实现任一集成）

3. **And** Rust 侧可有 **stub `trait SwarmOrchestrationPort`**（命名以实现为准）+ **单测**：默认实现 = 当前内置序曲路径

## 任务分解（Dev）

- [x] 起草 ADR（问题、决策、后果、合规 ADR-A）
- [x] 在 `agent-diva-swarm` 或组合层添加 trait + 默认实现挂钩点（避免循环依赖）
- [x] CI：`cargo tree` 仍满足 swarm 不依赖 meta

## 依赖

- **5.1–5.3** 任完成其一即可并行起草 ADR；**代码挂钩** 建议在 **5.1** 之后

---

## Dev Agent Record

### Implementation Plan

- ADR 单点落在 `agent-diva-swarm/docs/ADR_ORCHESTRATION_SPI_SHANNON_BRIDGE.md`（何时调用外部编排、DTO、FullSwarm 边界、降级策略、ADR-A）。
- `SwarmOrchestrationPort` + v0 DTO + `BuiltinSwarmOrchestrationPort` 与 `DEFAULT_SWARM_ORCHESTRATION_PORT`；`minimal_turn` 的 FullSwarm 分支经端口调用收敛（与原先 `execute_full_swarm_convergence_loop` 行为一致）。
- CI：`scripts/ci/check_swarm_no_meta.py` + `just check-swarm-no-meta` + `rust-check` job 一步。

### Debug Log

- Windows 上 `cargo tree` 输出非 UTF-8 导致 `subprocess` 解码失败；脚本改为 `encoding=utf-8, errors=replace`。

### Completion Notes

- ✅ AC1–2：ADR 含决策备忘表（Rust 扩展 → 侧车 → 嵌入式 Python 推荐顺序）。
- ✅ AC3：`SwarmOrchestrationPort` 默认实现与直接收敛循环等价；`orchestration_port` 模块单测覆盖无管道 / 有管道。
- ✅ CI 门禁：`agent-diva-swarm` 依赖树不得出现 `agent-diva-meta`。

## File List

- `agent-diva/agent-diva-swarm/src/orchestration_port.rs`（新建）
- `agent-diva/agent-diva-swarm/src/lib.rs`
- `agent-diva/agent-diva-swarm/src/minimal_turn.rs`
- `agent-diva/agent-diva-swarm/docs/ADR_ORCHESTRATION_SPI_SHANNON_BRIDGE.md`（新建）
- `agent-diva/scripts/ci/check_swarm_no_meta.py`（新建）
- `agent-diva/justfile`
- `agent-diva/.github/workflows/ci.yml`

## Change Log

- **2026-04-01：** Story 5.4 — ADR、编排 SPI trait、minimal_turn 挂钩、CI `cargo tree` 校验 ADR-A。
