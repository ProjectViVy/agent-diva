# feature-swarm-humanlike 分支接入 agent-diva-pro 可行性分析

> 调研日期：2026-06-11
> 关联文档：`docs/dev/awesomeagents/openharness-vs-diva-pro-gap-analysis.md`
> 调研目的：评估 `feature-swarm-humanlike` 分支的 swarm 能力接入 `agent-diva-pro` 的可能性与路径

---

## 一、调研背景

`openharness-vs-diva-pro-gap-analysis.md` 将 **Multi-Agent Swarm** 列为 agent-diva-pro 相比 OpenHarness 的**最大差距**（🔴）。`feature-swarm-humanlike` 分支曾实现过类似能力（cortex 大脑皮层、信箱、orchestration port），因此本调研评估该分支的可复用性。

---

## 二、分支历史与现状

### 2.1 分支定位
- **分支名**：`feature-swarm-humanlike`（也存在于 `agent-diva-pro` 仓库作为 `remotes/origin/feature-swarm-humanlike`）
- **核心提交**：`e67e629` "feat(swarm): 蜂群/大脑皮层 Epics 1-6 实现（swarm crate、GUI、manager、agent、CI）"
- **基础测试**：`cargo test -p agent-diva-swarm` 通过

### 2.2 与 pro HEAD 的距离
- **250 commits** 的 divergence（`e67e629` → `4c83fac`）
- **388+ 文件**差异，主要集中在 `agent-diva-agent`、`agent-diva-manager`、`agent-diva-gui`
- 期间 pro 经历了 4 次重大合并：sandbox、vrm-memory-test、context-compaction、planning epic

### 2.3 swarm crate 已消失
- 当前 `agent-diva-pro/HEAD` 的 workspace 中**已无 `agent-diva-swarm` 目录**
- 该目录仅在 `e67e629` 及其祖先提交中存在

---

## 三、swarm 分支提供了什么

`agent-diva-swarm` crate（位于 `e67e629`）包含以下核心模块：

| 模块 | 功能 | 对应 OpenHarness 能力 |
|------|------|---------------------|
| `cortex.rs` | 大脑皮层抽象，多 Agent 协调核心 | Coordinator 模式 |
| `orchestration_port.rs` | 编排端口，Agent 间调度接口 | Team Orchestration |
| `convergence.rs` | 收敛逻辑，多 Agent 达成一致 | Convergence Protocol |
| `process_events.rs` | 事件总线，Agent 间消息传递 | Mailbox |
| `execution_tier.rs` | 执行层抽象，in-process / subprocess 后端 | Dual Backend |
| `minimal_turn.rs` | 最小回合逻辑，Agent 交互原语 | Turn Primitive |

这些模块恰好对应 gap 分析中列出的 swarm 缺失能力。

---

## 四、接入 pro 的障碍分析

### 4.1 直接 merge 不可行
1. **架构漂移严重**：250 commits 间 pro 的 agent loop、manager、tool system 都经历了大规模重构
2. **类型冲突**：`SubAgentResult`、`AgentMode` 等类型在 pro 当前代码中已存在新定义
3. **依赖体系变化**：swarm crate 依赖的 `agent-diva-agent` API 与 pro HEAD 差异巨大
4. **Workspace 重组**：pro 当前 13 个 crate 的依赖图与 `e67e629` 时完全不同

### 4.2 功能重叠风险
- **Mask 系统（已部分实现）**：pro 当前正在开发 mask 系统（Epics 1-3），其中：
  - Epic 3 "parallel sub-agent orchestration"（`f172895`）与 swarm 的 orchestration 能力重叠
  - `AgentMode` 枚举可能与 swarm 的 execution_tier 冲突
- **PlanOrchestrator**（`agent-diva-agent`）：已有规划状态机，swarm 的 coordination 可能需要适配

### 4.3 测试与维护成本
- swarm 分支**通过基础测试**，但**未经过实际运行验证**
- 与 pro 当前的 SQLite WAL、Tool trait 统一、sandbox 审批等基础设施无集成
- Clippy 警告、文档缺失等债务需一并承接

---

## 五、推荐接入路径

### 策略 A：Cherry-pick 核心概念（推荐）
**不直接 merge，而是提取 swarm 的设计思想，重新实现到 pro 现有架构中。**

1. **提取** `cortex.rs` 的协调抽象 → 改造为 `MaskConfig` 的一种新 mode
2. **借鉴** `orchestration_port.rs` 的端口设计 → 扩展 `SubagentManager`
3. **复用** `process_events.rs` 的事件总线思想 → 与 `agent-diva-manager` 的事件系统集成
4. **适配** `execution_tier.rs` 的双后端 → 接入 pro 的 sandbox 审批流程

**优点**：避免 250 commits 的冲突，保留设计思想，与 mask 系统协同
**风险**：实现成本较高，需要深入理解两套架构

### 策略 B：完整分支移植
**以 `e67e629` 为基础，将 swarm crate 移植到一个新的命名空间。**

1. 新建 `agent-diva-swarm-v2` crate
2. 复制 swarm 分支的 7 个核心模块
3. 逐个适配 pro HEAD 的依赖与类型
4. 通过 `Mask` 系统的 `AgentMode::Swarm` 模式暴露

**优点**：保留完整实现，参考价值高
**风险**：适配工作量大（估计 200-400 工时），可能与 mask Epic 3 冲突

### 策略 C：混合策略（最务实）
**保留 swarm 分支作为参考，新功能在 mask 系统下增量实现。**

1. swarm crate 保留在 `e67e629` 作为**设计参考**
2. mask 系统 Epic 3 已经覆盖了部分 swarm 能力，继续推进
3. 后续根据实际需求，**逐个**将 swarm 的 cortex、convergence 等概念引入 pro
4. 不强求一次性接入所有 swarm 能力

**优点**：风险可控，与现有路线一致
**风险**：可能遗漏 swarm 特有的设计（如 worktree 隔离）

---

## 六、优先级建议

基于 gap 分析的 P0 定位与当前 mask 系统进度：

| 优先级 | 工作项 | 理由 |
|--------|--------|------|
| **P0** | 完成 mask Epic 3 已有工作 | 已投入开发，不应被 swarm 分支打断 |
| **P1** | 从 swarm 提取 cortex 概念文档化 | 作为后续多 Agent 协调的设计输入 |
| **P1** | 设计 Swarm Mask mode | 在 mask 框架下预留 swarm 能力入口 |
| **P2** | 实现 in-process 后端抽象 | 对应 `execution_tier.rs` 的 in-process backend |
| **P2** | 实现 Agent 间事件总线 | 对应 `process_events.rs`，需与 manager 事件系统集成 |
| **P3** | 完整移植 swarm crate | 仅在 P1/P2 验证价值后实施 |

---

## 七、具体操作建议

### 7.1 立即可做（不阻塞 mask Epic 3）
1. **阅读 swarm 分支源码**（`e67e629` 的 `agent-diva-swarm/src/`），作为 mask 系统的设计参考
2. **对比** swarm 的 `AgentMode` 与 mask 的 `AgentMode` 枚举，识别可融合点
3. **记录** swarm 的 cortex 抽象到 `docs/dev/mask-system/cortex-design-notes.md`

### 7.2 短期（1-2 周）
1. 在 mask 系统的 `AgentMode` 枚举中**预留** `Swarm` 变体
2. 在 `SubagentManager` 中**预埋**事件总线 hook 点
3. 创建 RFC 文档 `docs/dev/rfcs/swarm-integration.md`

### 7.3 中期（1-2 月）
1. 选择策略 A 或 C，**先实现 in-process 后端**（最小可用特性）
2. 通过 mask 的 `AgentMode::Swarm` 暴露给用户
3. 收集反馈后再决定是否完整移植 cortex 架构

---

## 八、风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|----------|
| 与 mask Epic 3 实现冲突 | 🟡 中 | 先完成 mask Epic 3 基础功能，再引入 swarm |
| 250 commits divergence 导致 cherry-pick 失败 | 🟡 中 | 不采用直接 cherry-pick，改用设计参考方式 |
| 用户期望一次性获得完整 swarm 能力 | 🟡 中 | 通过 RFC 明确分阶段交付路径 |
| swarm 分支代码质量未达 pro 标准 | 🟢 低 | 仅作为设计参考，不直接合入代码 |
| Git worktree 隔离能力难以实现 | 🔴 高 | 当前 pro 架构下建议简化为命名空间隔离 |

---

## 九、结论

### 核心结论
1. **直接 merge `feature-swarm-humanlike` 不可行**：250 commits 的架构漂移会导致不可调和的冲突
2. **swarm 分支有参考价值**：其 cortex、orchestration、event bus 的设计思想应被吸收
3. **接入路径已经存在**：mask 系统的 Epic 3 正在实现并行 sub-agent 编排，建议在此基础上扩展而非另起炉灶
4. **推荐策略 C（混合策略）**：保留 swarm 分支作为设计参考，在 mask 框架下增量引入 swarm 能力

### 一句话总结
> **不要 merge 旧分支，要吸收旧思想。** swarm 分支是设计参考而非代码资产，应通过 mask 系统的演进路径将 swarm 能力逐步接入 pro。

---

## 十、附录

### A. 关键文件位置

| 内容 | 路径 |
|------|------|
| gap 分析文档 | `docs/dev/awesomeagents/openharness-vs-diva-pro-gap-analysis.md` |
| swarm crate 源码 | `~/Desktop/morediva/agent-diva-swarm/agent-diva-swarm/src/` |
| 关键提交 `e67e629` | `git checkout e67e629`（在 worktree 中） |
| mask 系统设计 | `agent-diva-pro/docs/dev/mask-system/` |
| 当前 pro HEAD | `4c83fac` (backup/pro-pre-push-2026-06-11) |

### B. swarm 提交摘要

```
18fe139 feat(swarm): add agent-diva-swarm workspace member and crate
... (中间 6 个 epic commits)
e67e629 feat(swarm): 蜂群/大脑皮层 Epics 1-6 实现（swarm crate、GUI、manager、agent、CI）
```

### C. 参考文档

| 文档 | 路径 |
|------|------|
| OpenHarness 深度调研 | `docs/dev/awesomeagents/openharness-analysis.md` |
| 能力清单 | `docs/dev/awesomeagents/diva-capability-checklist.md` |
| 演进路线 | `docs/dev/awesomeagents/evolution-roadmap.md` |
| 未知缺陷分析 | `docs/dev/awesomeagents/unknown-deficits.md` |

---

> 生成日期：2026-06-11
> 调研人：松本 (Hermes Agent)
> 状态：已完成，等待用户决策（选择策略 A/B/C）
