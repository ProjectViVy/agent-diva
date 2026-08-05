# agent-diva 审批模型调研：Claude Code 对比与「智能、谨慎、信任」完善方案

- 日期：2026-08-05
- 范围：Claude Code 权限/审批模型调研；agent-diva 现状盘点；差距分析；三模式窗口完善方案
- 结论摘要：Claude Code 有 **6 种权限模式 + 一套规则系统（allow/ask/deny）**；
  agent-diva 现有 3 个 GUI 模式映射到 4 个后端策略，但「信任」与「谨慎」行为几乎相同、
  「智能」缺少风险分级预判，且 Guardian 自动放行能力在生产路径全部默认关闭 —— 确实"半残"。
  本文给出对齐方案：把「智能、谨慎、信任」实现为真正有差异的三档审批窗口。

---

## 1. Claude Code 审批模型全览

### 1.1 六种权限模式（Permission Modes）

Claude Code 提供 6 种权限模式，从最严格到最宽松：

| 模式 | 英文 | 行为 | 适用场景 |
| --- | --- | --- | --- |
| 计划 | `plan` | 只读探索模式：只能分析、规划，禁止任何编辑/命令副作用 | 需求分析、方案设计 |
| 严格白名单 | `dontAsk` | 仅允许预先批准的（白名单）工具/命令，其余**静默拒绝** | 受控 CI、安全环境 |
| 默认 | `default` | 编辑与命令逐项询问用户（最基础的交互模式） | 默认日常使用 |
| 接受编辑 | `acceptEdits` | 文件编辑**自动批准**，但执行命令仍需确认 | 大量改代码、少跑命令 |
| 自动 | `auto` | **AI 分类器**判断：低风险自动放行，高风险才询问（"两阶段分类审判"） | 效率与安全的平衡点 |
| 绕过权限 | `bypassPermissions` | 跳过全部检查，无任何提示（官方仅建议在隔离环境使用） | 完全可信/隔离环境 |

要点：
- `default` / `acceptEdits` / `plan` / `bypassPermissions` 是基础四档（官方文档经典划分）；
  `auto` 与 `dontAsk` 是后加入的两档，分别对应"AI 智能放行"与"白名单严格模式"。
- `auto` 模式使用 AI 分类器对每次工具调用做风险评估（低风险 → 自动执行；高风险 → 询问），
  相当于把"风险分级"内建进权限系统，而非简单的"全问 / 全放"。
- `bypassPermissions` 语义是"完全信任"，没有任何安全检查，官方警告仅限隔离环境。

### 1.2 工具级规则系统（settings.json `permissions`）

在 `settings.json` 中可配置逐工具/逐命令规则：

| 配置项 | 含义 |
| --- | --- |
| `permissions.allow` | 自动放行的规则，如 `"Bash(git status *)"`（支持通配符） |
| `permissions.ask` | 强制询问的规则 |
| `permissions.deny` | 直接拒绝的规则（**优先级最高**，命中即拒绝） |
| `permissions.additionalDirectories` | 额外授予访问的目录 |
| `permissions.defaultMode` | 会话默认权限模式（如 `"acceptEdits"`） |

- **优先级**：`deny > ask > allow`，且**首个匹配生效**（first match wins）。
- 规则与模式是两层：模式决定"没有规则命中时"的默认行为，规则决定"命中时"的精确行为。
- 会话内可通过 `/permissions` 等命令查看/调整当前模式与规则。
- 注意：deny 规则不阻止子进程的间接访问（如脚本内绕行），敏感文件保护需靠 sandbox。

### 1.3 与 agent-diva 的映射直觉

| Claude Code | agent-diva 对应物 |
| --- | --- |
| `plan`（只读） | 消息发送 mode `ask`（Ask mode，只读不落盘） |
| `default`（全问） | 谨慎（cautious）→ `OnRequest` |
| `auto`（AI 分级） | 智能（smart）→ 目标形态 |
| `acceptEdits`（编辑自动） | 无对应（工具粒度可后续对齐） |
| `dontAsk`（白名单严格） | 无对应（规则系统已有雏形） |
| `bypassPermissions` | `Never` 策略（存在但 GUI 未暴露） |

---

## 2. agent-diva 审批模型现状

### 2.1 后端策略枚举（4 档）

`agent-diva-sandbox/src/policy.rs:197`：

```rust
pub enum AskForApproval {
    Never,          // 从不请求审批，总是直接执行
    OnFailure,      // 默认：沙箱执行失败后才请求审批
    OnRequest,      // LLM/策略决定是否请求审批（询问）
    UnlessTrusted,  // 未信任命令才请求审批
}
```

### 2.2 GUI 三模式映射

`agent-diva-gui/src/components/ChatView.vue:592` 定义三模式：

| GUI 模式 | 文案 | 发送 approvalPolicy | 后端策略 |
| --- | --- | --- | --- |
| 谨慎 | 所有操作均需确认 | `on-request` | `OnRequest` |
| 智能 | 低风险自动放行 | `on-failure` | `OnFailure` |
| 信任 | 仅高风险需确认 | `unless-trusted` | `UnlessTrusted` |

映射在 `agent-diva-gui/src/App.vue:1387`；后端解析别名在 `agent-diva-manager/src/handlers.rs:87`（`parse_approval_policy` 同时接受 `cautious/smart/trusted` 与 `on-request/on-failure/unless-trusted/never`）。

### 2.3 判定链（一次命令执行如何决定是否审批）

1. **规则评估** `exec_policy.rs:256 get_approval_requirement`：
   - `Decision::Forbidden`（deny 规则）→ `Forbidden`，直接拒绝；
   - `Decision::Prompt`（ask 规则）→ `Never` 下拒绝，否则 `NeedsApproval`；
   - `Decision::Allow` 且**有规则匹配** → `Skip`（所有模式一致放行）；
   - `Decision::Allow` 且**无规则匹配** → 按策略：`Never`/`OnFailure` → `Skip`；
     `OnRequest`/`UnlessTrusted` → `NeedsApproval`。
2. **Guardian 复核** `guardian.rs:330 DefaultGuardianReviewer::review`：
   - `Forbidden` → Denied；
   - `can_skip` → Defer（放行）；
   - `Never`：known-safe（命中 Allow 规则）自动批准、只读命令自动批准、危险命令拒绝、其余自动批准（可开自动学习）；
   - `OnFailure` → **直接 Defer（无任何风险预判）**；
   - `OnRequest | UnlessTrusted` → **同一分支**：known-safe/只读自动批准、危险命令必问、未知命令必问。
3. 需要审批时进入 `CommandApprovalCoordinator` → governance ledger → GUI 统一审批中心（Drawer）。

### 2.4 Guardian 配置

`guardian.rs:26 GuardianConfig`：`auto_approve_known_safe`（默认 **false**）、`auto_approve_read_only`（默认 **false**）、`enable_auto_learning`（默认 **false**）、熔断参数等。提供了 `strict()`/`liberal()` 构造器，但**生产路径** `orchestrator.rs:885` 固定使用 `GuardianConfig::default()`，不随 GUI 模式切换。

### 2.5 规则管理

- 后端：命令规则存储（allow/deny pattern），API `get_command_rules / set_command_rule_enabled / delete_command_rule`（`capabilities.ts:52`）。
- GUI：`SandboxSettingsSection.vue` 可查看/启用/删除规则，并配置 `deny_patterns` 文本列表。
- 危险判定：`guardian.rs:284 is_potentially_dangerous`（sudo/rm/内联 shell 等黑名单）；
  只读判定：`guardian.rs:231 appears_read_only`（git status/log/diff 等白名单）。

### 2.6 其他

- 模式不持久化：`ChatView.vue:191 permissionMode` 默认 `'smart'`，无本地存储，重启回默认。
- Ask mode（只读）已存在：`agent-diva-agent/src/agent_loop/turn/prompt.rs:61`。
- 计划域审批（PlanApprovalCard）与命令域审批（统一 Drawer）已分离（v0.5.1/v0.5.2 迭代）。

---

## 3. 差距分析：为什么"半残"

### 3.1 实锤差距

| # | 差距 | 证据 | 影响 |
| --- | --- | --- | --- |
| G1 | **信任 ≡ 谨慎**：两个模式后端行为完全相同 | `guardian.rs:369` 同一分支；`exec_policy.rs:299` 同一处理 | 「信任」文案是"仅高风险需确认"，实际与谨慎无差别，用户选择信任也没有更流畅的体验 |
| G2 | **智能 = 盲跑**：无风险预判，先执行、失败才问 | `guardian.rs:365-367` `OnFailure => Defer`（跳过 known-safe/只读/危险检查） | 危险命令也会先跑沙箱再问；与文案"低风险自动放行"不符（应当"低风险放行、高风险先问"） |
| G3 | **自动放行能力生产默认全关** | `orchestrator.rs:885` 固定 `GuardianConfig::default()`；三个开关默认 false；`strict()/liberal()` 仅测试使用 | 即使 OnRequest 分支写了 known-safe/只读自动批准，生产中也从不生效 |
| G4 | **UnlessTrusted 语义未实现** | 无"信任命令集合"的判定；仅 Allow 规则（对所有模式都生效） | 信任模式没有专属行为 |
| G5 | **模式不持久化** | `ChatView.vue:191` 无存储 | 每次启动回退到智能，用户设置丢失 |
| G6 | **无 acceptEdits 档** | 无按工具类型（编辑/命令）分流的粒度 | 编辑密集场景缺少"只放行编辑"的档位 |
| G7 | **auto（AI 分级）缺失** | guardian 的 is_known_safe/read-only/dangerous 是**静态规则**，无 AI 分类器参与 | 智能模式达不到 Claude `auto` 的智能度（可选增强） |

### 3.2 优势（保留）

- deny 规则硬拒绝 + ask 规则 + allow 规则已存在，优先级（Forbidden > Prompt > Allow）正确；
- 统一审批中心（Drawer）+ governance ledger 架构清晰（v0.5.x 迭代成果）；
- 沙箱失败重试审批链路（`allows_sandbox_failure_retry`）可用于 OnFailure 语义。

---

## 4. 完善方案：「智能、谨慎、信任」三窗口

目标：让三个 GUI 模式成为**行为明确不同、逐步放松**的三档审批窗口，对齐 Claude Code 的
`default → auto → dontAsk/acceptEdits` 谱系。

### 4.1 模式语义（目标行为）

| 模式 | 对标 Claude | 行为定义（按优先级） |
| --- | --- | --- |
| 谨慎 | `default` 强化 | deny 规则硬拒绝；**所有**命令/工具先询问（含只读）；危险命令标记风险 |
| 智能 | `auto`（静态版） | deny 硬拒绝；Allow 规则 + 只读命令**自动放行**；危险命令询问；未知命令询问 → "低风险自动放行"落地 |
| 信任 | `dontAsk` + `acceptEdits` 融合 | deny 硬拒绝；Allow 规则 + 只读自动放行；**自动学习**用户批准的规则（enable_auto_learning）；未知命令**放行**，仅危险命令询问 → "仅高风险需确认"落地 |

### 4.2 技术改动清单

| 步骤 | 位置 | 改动 |
| --- | --- | --- |
| 1 | `guardian.rs:365` | `OnFailure` 分支不再直接 Defer：改为复用风险预判（known-safe/只读自动放行、危险必问、未知询问）。若需保留"失败后审批"语义，可拆成 `OnFailure` 仅用于"沙箱失败重试审批"（exec_policy 层），guardian 层按 OnRequest 语义走 |
| 2 | `guardian.rs:369` | 拆分 `OnRequest` 与 `UnlessTrusted`：`UnlessTrusted` = 只读/known-safe 自动 + 危险询问 + **未知放行**；`OnRequest` = 只读/known-safe 自动 + 危险询问 + **未知询问** |
| 3 | `orchestrator.rs:885` | `GuardianConfig` 按模式注入：谨慎 → `strict()`（全关）；智能 → `auto_approve_known_safe + auto_approve_read_only` 开启、learning 关；信任 → liberal（含 `enable_auto_learning: true`）。需要把 approval_policy 从 handler 传到 orchestrator 的 config 选择逻辑 |
| 4 | `manager/handlers.rs` | `send_message` 携带的 `approval_policy` 同时推导 `GuardianConfig` 档位（strict/balanced/liberal），传给 sandbox orchestrator |
| 5 | GUI `ChatView.vue` | `permissionMode` 持久化（localStorage/config），启动恢复 |
| 6 | 测试 | 为三模式分别写行为契约测试：谨慎全问、智能只读放行/危险询问、信任未知放行/危险询问/自动学习 |
| 7 | 文案 | 三模式 desc 微调以准确描述行为（谨慎=全部确认；智能=低风险自动放行；信任=仅危险需确认） |

### 4.3 可选项（后续迭代）

- **acceptEdits 档**：新增第四档或在智能档内按工具类型分流（编辑自动批准、命令询问）。
- **AI 分级**：用 LLM 对未知命令做一次性风险评分（低危自动放行、高危询问），替代/增强静态规则。
- **规则管理增强**：GUI 内直接添加 Allow 规则（当前只能启用/删除既有规则）。

---

## 5. 建议实施顺序

1. **P0（本次文档后）**：步骤 1-2（guardian 分支语义修正：OnFailure 预判 + UnlessTrusted 独立）+ 步骤 3-4（config 按模式注入）——这三点直接消除 G1/G2/G3/G4。
2. **P1**：步骤 5（模式持久化，消除 G5）+ 步骤 6（三模式契约测试）。
3. **P2**：可选项（acceptEdits 档、AI 分级、规则管理增强）。

---

## 6. 参考资料

- Claude Code 权限模式官方文档（zh-CN）：https://code.claude.com/docs/zh-CN/permission-modes
- Claude Code 权限配置（runoob）：https://www.runoob.com/claude-code/claude-code-permission.html
- Claude Code 权限配置（w3cschool）：https://www.w3cschool.cn/aicodingguide/claude-code-permissions.html
- Claude Code 的六种授权模式（CSDN）：https://blog.csdn.net/jarvisuni/article/details/161348107
- Claude Code 权限模式完全指南（腾讯云）：https://cloud.tencent.com/developer/article/2667942
- Claude Code Auto Mode（claudefa.st）：https://claudefa.st/blog/guide/development/auto-mode

## 7. 附录：本文引用的 agent-diva 源码位置

- `agent-diva-sandbox/src/policy.rs:197-228`（AskForApproval 枚举与辅助方法）
- `agent-diva-sandbox/src/exec_policy.rs:256-309`（get_approval_requirement）
- `agent-diva-sandbox/src/guardian.rs:26-84`（GuardianConfig）、`330-389`（review 分支）、`221-330`（known-safe/read-only/dangerous 判定）
- `agent-diva-sandbox/src/orchestrator.rs:885-886`（生产路径 GuardianConfig::default()）
- `agent-diva-gui/src/components/ChatView.vue:191,592-595`（permissionMode 与三模式定义）
- `agent-diva-gui/src/App.vue:1331,1387-1394`（模式→approvalPolicy 映射）
- `agent-diva-manager/src/handlers.rs:87-103,1490-1504`（approval_policy 解析与测试）
