# 沙箱审批策略 + HITL 完善提案

- 日期：2026-08-05
- 类型：实现前提案（本回合仅文档归档，无代码变更）
- 状态：提案已评审归档；P0–P2 实现待开单独回合
- 相关调研：[`approval-model-claude-code-vs-agent-diva.md`](./approval-model-claude-code-vs-agent-diva.md)
- 参考源码：`morediva/.workspace/claude-code/`、`morediva/.workspace/learn-claude-code/s03_permission/`
- backlog：`TODOLIST.md` → `审批三模式完善`、`Approval dual-channel unification`

---

## 目标

让 agent-diva 的沙箱真正知道 **何时该触发审批、何时不该**，并让 GUI「谨慎 / 智能 / 信任」三模式产生可观测的行为差异。对标 Claude Code（`morediva/.workspace/claude-code`）的权限决策管线，而不是简单照搬 UI 文案。

## 现状结论（半残证据）

既有调研：[`approval-model-claude-code-vs-agent-diva.md`](./approval-model-claude-code-vs-agent-diva.md)（2026-08-05）。复核 **生产路径** 后，问题比该文档 G3 表述更重：

### 生产执行链（实际生效）

```
GUI permissionMode
  → App.vue 映射 approval_policy
  → manager/handlers parse_approval_policy → metadata
  → agent_loop.apply_approval_policy_from_metadata
  → tool_assembly.with_approval_policy
  → ExecTool.with_approval_backend
       ToolOrchestrator::new(manager, policy)  // 无 Guardian、无 ExecPolicy
  → orchestrator.run
       check_approval → resolve_initial_override
  → ApprovalRequired? → CommandApprovalCoordinator.request (HITL)
  → record ApprovedOnce → 重跑
```

### 核心缺陷

| ID | 问题 | 证据 | 用户感知 |
| --- | --- | --- | --- |
| **G1** | **信任 ≡ 谨慎** | `exec_policy.rs` / `orchestrator.check_approval` / `resolve_initial_override` 把 `OnRequest \| UnlessTrusted` 绑在同一分支 | 「仅高风险确认」与「全部确认」无差别 |
| **G2** | **智能 = 先跑再问** | `OnFailure` → 首次 `Skip` + 沙箱内执行；仅失败升级时 `ApprovalRequired`（`resolve_initial_override` Never/OnFailure 分支） | 文案是「低风险自动放行」，实为「几乎不预审」 |
| **G3'** | **Guardian 未接入生产** | `agent-diva-tools/src/shell.rs` 中 `with_approval_backend` 仅 `ToolOrchestrator::new`；全仓无生产路径 `with_guardian` | known-safe / 只读 / 危险预判 **死代码** |
| **G3''** | **ExecPolicy 未接入生产** | 同上，无 `with_exec_policy`；GUI 规则管理与运行时脱节 | Allow/Deny 规则对 shell 路径基本无效 |
| **G4** | **UnlessTrusted 语义缺失** | 无「未知命令可放行」分支 | 信任模式无法变流畅 |
| **G5** | 模式不持久化 | `ChatView.vue` `permissionMode` 默认 smart | 重启丢设置 |
| **G6** | 自动学习 stub | `orchestrator` create_rule 仅 debug log | 信任模式「越用越少问」不成立 |
| **G7** | HITL 通道仍有双通道残留 | TODOLIST `Approval dual-channel` | 偶发丢/重事件（相对次要） |

> 先前调研 G3 引用 `orchestrator.rs` 中 `GuardianConfig::default()` 实际是 **测试路径**；真正问题是生产 **根本不挂 Guardian**。

### 与 Claude Code 的关键差距

CC 源码（`.workspace/claude-code`）决策链要点：

1. **deny → ask → 工具自检 → 内容级 ask / 安全路径（bypass 免疫）→ 模式 allow → 整工具 allow → passthrough 兜底 ask**（`permissions.ts` `hasPermissionsToUseToolInner`）
2. 规则三值 `allow | deny | ask`，模式决定 **无规则命中时的默认**
3. `auto` 用分类器 LLM（`yoloClassifier.ts`）做风险分级；失败回退询问
4. 模式与规则正交；显式 deny/ask 不因 bypass 失效

agent-diva 目标不需要一次搬完 6 模式 + AI 分类器，但必须先让 **三模式默认行为可区分**，并把规则/预判接到生产路径。

## 目标语义（三窗口）

| GUI | 后端策略 | 目标行为（按优先级） | 对标 CC |
| --- | --- | --- | --- |
| **谨慎** | `OnRequest` | deny 硬拒绝；**默认全问**（含只读）；已缓存 session 批准可跳过 | `default` |
| **智能** | 建议保留枚举名 `OnFailure` 或新增语义别名，但 **改默认行为** | deny 硬拒绝；Allow 规则 + **只读 / known-safe 自动放行**；危险 **先问**；未知 **先问**；沙箱失败后仍可升级审批重试 | 静态版 `auto` |
| **信任** | `UnlessTrusted` | deny 硬拒绝；Allow + 只读/known-safe 自动放行；**未知自动放行**；**仅危险先问**；可选自动学习 Allow 规则 | `dontAsk` 宽松 + 危险兜底 |

说明：

- **不要**再把「智能」实现成「无预判盲跑」；沙箱失败重试仍保留为 **第二道闸**（`allows_sandbox_failure_retry`），不是唯一闸。
- `Never` 继续存在（CLI/高级配置），GUI 可不暴露；语义对齐 CC `bypassPermissions`（仍应尊重 Forbidden/deny）。

## 决策管线（目标形态）

对每一次 shell/exec 调用，统一经过：

```
1. deny / Forbidden 规则        → Deny（硬拒绝，模式无关）
2. 会话缓存已批准               → Allow（scope: once/session）
3. ask 规则                     → Ask（HITL）
4. allow 规则 / known-safe      → Allow（智能/信任；谨慎可选关闭）
5. 静态风险分类
     - dangerous                → Ask（所有非 Never 模式）
     - read_only / low-risk     → Allow（智能/信任）Ask（谨慎）
     - unknown                  → Ask（谨慎/智能）Allow（信任）
6. 执行（默认沙箱）
7. 沙箱失败 + 策略允许重试      → Ask 升级（无沙箱重试）→ HITL
```

对齐 CC 的精神：**显式规则优先于模式；危险/安全路径可「模式免疫」**。

## 后续实现蓝图

以下阶段描述「若批准后如何落地」；**归档本提案时未执行实现**。

### Phase 0 — 契约与接线（P0，实现时优先）

**目的**：生产路径真正拥有「能否跳过审批」的判定器。

1. **统一审批预判入口**
   - 在 `agent-diva-sandbox` 抽出清晰 API，例如 `ApprovalGate::evaluate(command, cwd, policy, config) → Allow | Ask | Deny`
   - 内部合并：`ExecPolicy` 评估 + 静态启发式（现有 `DefaultGuardianReviewer` 的 known-safe / read-only / dangerous）+ 策略分支
   - 避免 `exec_policy` 与 `guardian` 两套平行逻辑继续分叉

2. **拆分模式分支**
   - `OnRequest`：未知 → Ask；只读/known-safe 仅在 `auto_approve_*` 打开时放行（谨慎档默认全关）
   - `UnlessTrusted`：未知 → Allow；危险 → Ask
   - `OnFailure`（智能）：**首次也做风险预判**（危险/未知 Ask；低风险 Allow）；失败后升级逻辑保留
   - `Never`：尽量 Allow，但 Forbidden/危险可 Deny 或仍 Ask（建议：Forbidden 拒，危险 Deny 或 Ask 二选一并写死契约）

3. **生产接线**（关键缺口）
   - `ExecTool::with_approval_backend`：创建带 Guardian/ExecPolicy 的 orchestrator**
     - 加载规则来源：sandbox settings 中的 deny/allow（与 GUI `SandboxSettingsSection` 同源）
     - `GuardianConfig` 按 `AskForApproval` 映射：
       - cautious/`OnRequest` → `strict()`
       - smart/`OnFailure` → known-safe + read-only 开、learning 关
       - trusted/`UnlessTrusted` → liberal（含 learning，若 Phase 0 学习未就绪则先开 auto_approve 两开关）
   - `tool_assembly` / agent 重建 registry 时随 policy 重建或热更新 gate 配置

4. **HITL 重跑路径不变**
   - 保持 `ApprovalRequired` → coordinator → `ApprovedOnce/Session` → re-run
   - 确保 gate 尊重缓存决策（避免「批准后仍再问」）

5. **契约测试**（crate 级，不依赖 GUI）
   - 谨慎：`git status` / `echo` / `rm -rf` 均 Need Ask（或 rm 走 deny pattern 直接拒）
   - 智能：`git status` Allow；`rm` / `sudo` Ask；未知如 `curl ...` Ask
   - 信任：`git status` Allow；未知 `make test` Allow；`sudo`/`rm` Ask
   - 三模式 + 同一 Allow 规则 / Deny 规则
   - 沙箱失败升级：智能模式下低风险首次 Allow，模拟 sandbox deny 后仍 Need Ask

**实现时主要文件**

- `agent-diva-sandbox/src/guardian.rs`
- `agent-diva-sandbox/src/exec_policy.rs`
- `agent-diva-sandbox/src/orchestrator.rs`
- `agent-diva-sandbox/src/policy.rs`（补文档与 helper，必要时加 `for_mode` config factory）
- `agent-diva-tools/src/shell.rs`（生产接线）
- 可选：`agent-diva-agent/src/tool_assembly.rs`（规则路径/config 注入）

### Phase 1 — 体验闭环（P1）

1. GUI `permissionMode` 持久化（localStorage 或 profile config）
2. 三模式文案与真实行为对齐（ChatView 模式 desc）
3. 审批理由透传到 Drawer（为何问：dangerous / unknown / rule / sandbox-escalation）
4. 模式切换后对 **后续** 工具调用立即生效（agent_loop 已有 metadata 应用，确认热切换不建脏 registry）
5. 文档：更新 research 的 G3 表述；迭代日志四件套

### Phase 2 — 能力增强（P2，可选）

1. **内容级规则**（CC `Bash(npm publish:*)`）：规则支持 argv/子命令匹配，不仅前缀
2. **acceptEdits 粒度**：文件系统写工具与 shell 分流（编辑自动 / 命令询问）
3. **自动学习落地**：用户 ApproveSession/Global 时 `append_amendment` 真正写规则（修 stub）
4. **AI 分级（可选）**：仅智能模式对 unknown 调轻量分类器；失败回退 Ask（对齐 CC `auto` + yoloClassifier）
5. 双通道 SSE 合并 + 超时倒计时（既有 TODOLIST 项）
6. 子 agent 权限冒泡（CC `bubble`）

## 明确不做（提案归档回合）

| 类别 | 内容 |
| --- | --- |
| **提案回合** | 不改 `.rs`/`.vue`/生产配置；不写实现测试；不接线 |
| **实现时也不优先** | 完整 6 模式 UI；CC 8 来源规则合并；依赖真实 LLM 的 auto 分类器作为默认 |

## 风险与兼容（供评审）

| 风险 | 缓解 |
| --- | --- |
| 智能模式从「几乎不问」变为「未知也问」→ 更频繁 HITL | 预期行为修正；文案同步；只读/known-safe 放行覆盖日常命令 |
| 信任模式未知放行 → 安全面扩大 | 危险启发式 + deny 规则兜底；保护路径（.git/.env）保持只读沙箱 |
| 规则文件缺失时行为回退 | 无规则时完全依赖静态启发式 + 模式默认 |
| Windows 命令形态（`powershell`/`cmd`）启发式误判 | 启发式以 argv[0] 与常见子命令为主；补充 Windows 用例测试 |

## 实现时验证清单

- `cargo test -p agent-diva-sandbox`（gate 契约）
- `cargo test -p agent-diva-tools`（ExecTool HITL 重跑 + 模式差异）
- 完整 `just fmt-check && just check && just test` 在落地提交前
- 手工 smoke：GUI 三模式各跑 `git status`、危险命令、未知命令
- 迭代日志：`summary/verification/release/acceptance`

## 实现时建议顺序

1. 写失败契约测试（红）证明当前 信任≡谨慎、智能无预判
2. 抽出/修正 `ApprovalGate` + 拆分三模式
3. `ExecTool` 生产接线 Guardian+ExecPolicy+Config 映射
4. 测试转绿
5. GUI 持久化 + 文案
6. 提交 + 更新 TODOLIST「审批三模式完善」

## 与既有调研的关系

| 文档 | 角色 |
| --- | --- |
| [`approval-model-claude-code-vs-agent-diva.md`](./approval-model-claude-code-vs-agent-diva.md) | Claude Code 对照调研、G1–G8、初步三窗口方案 |
| **本文** | 生产路径复核（G3'/G3''）、目标决策管线、可执行 P0–P2 蓝图归档 |

实现时请以本文 **G3'/G3''** 为准：生产未挂 Guardian/ExecPolicy，而非仅 `GuardianConfig::default()` 开关问题。

## 参考

- 调研：`docs/research/approval-model-claude-code-vs-agent-diva.md`
- CC：`.workspace/claude-code/src/types/permissions.ts`、`src/utils/permissions/permissions.ts`、`yoloClassifier.ts`
- 教学：`.workspace/learn-claude-code/s03_permission/`
- backlog：`TODOLIST.md` → `审批三模式完善`、`Approval dual-channel unification`
