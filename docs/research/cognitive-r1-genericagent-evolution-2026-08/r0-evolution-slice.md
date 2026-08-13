# R0 Evolution 切片盘点（非完整 R0）

- 状态：`Research Draft / Source-backed`
- 日期：2026-08-13
- 范围：**仅 Evolution 表面**（AutoDream、Skill 触点、Proposal、Governance、GUI）
- **完整 R0 缺失声明：** Persona 全量、BML CRUD 全景、STM/checkpoint、memory_md 删除影响、全仓 bus 不在此文

产品约束：`evolution-genericagent-reset-2026-08/decision-record.md` D1–D4；EPIC Evolution = Skill 演进。

## 1. 一句话现状

今日 Diva「Evolution」= **AutoDream 运行管理 + Laputa 提案收件箱 + Governance 决策/应用 + Changelog 审计 + SelfEvolution 策略 + Recall 反馈**。  
**不是** Skill 形成/验证/启停/版本中心。Skill 由 `SkillsLoader` 读 `skills/*/SKILL.md`，与 Proposal 状态机几乎无桥接。

## 2. 模块地图

### 2.1 `agent-diva-core::evolution`

| 符号 | 角色 |
| --- | --- |
| `EvidenceRef` / `EvidenceSource` | 提案证据指针 |
| `ProposalType` | MemoryPatch, LearningNote, IdentityPatch, RelationshipUpdate, CommitmentSet, **SopCreate**, Deprecation |
| `ProposalState` | PendingReview…Applied/Reverted/… |
| `EvolutionProposal` | 治理信封 |
| `AutoDreamRunRecord` / phases | 运行生命周期 |
| `MemoryCandidate` | 反思候选 |
| `SopCreate.target_section()` | → **Identity**（与 Skill 对象无关） |

**源码事实：** AutoDream `CandidateGate` **拒绝** `SopCreate`（UnsupportedType）。类型暗示 SOP，运行时不产出技能晋升。

### 2.2 AutoDream crate

流水线：`Queued → Gathering → Reflecting → Validating → Publishing → Completed|Failed|Cancelled`  
阶段：Orient / Gather / Consolidate / Propose  
受限：可读 session/Laputa；写 autodream 输出；经 API 建提案；禁 shell/直写权威。  
实践候选偏 `MemoryPatch` / `LearningNote` / `Deprecation`。

### 2.3 Laputa / Governance

| 路径 | 职责 |
| --- | --- |
| `proposals.rs` | 文件优先提案 CRUD / apply |
| `governed_apply.rs` | `MemoryGovernanceCoordinator` ↔ approval ledger |
| `.laputa/governance.sqlite3` | 审批映射 |
| `.laputa/proposals/` | 提案 JSON |

错误串 `governance ledger failed: …` 与桌面失败基线一致。

### 2.4 Manager 路由

| 组 | Path 摘要 |
| --- | --- |
| AutoDream | `/api/autodream/runs[/:id[/live-text|events|cancel]]` |
| Laputa Evolution | `/api/laputa/proposals/**`, decision, apply, changelog, recall-feedback… |
| SelfEvolution | `/api/config/self-evolution` |
| Skills | `/api/skills`, `/api/skills/:name` |
| Approvals | `/api/approvals/**`（聊天 Approval Center） |

### 2.5 GUI

| 入口 | 说明 |
| --- | --- |
| `EvolutionView.vue` | 页签 inbox / runs / audit / policy |
| `ProposalInbox/Detail`, `GovernanceActionBar` | 治理动作 |
| `SelfEvolutionSettings.vue` | 频率/阈值/确认策略 |
| `SkillsSettings.vue` | **分离** 的技能安装 |
| Approval deep-link | `openEvolutionProposal` |

文案硬编码把 personality/memory/SOP/skill 捆在同一审批叙事 → 与 D1/D3 冲突。

### 2.6 Skill runtime（脱钩）

- `agent-diva-agent/src/skills.rs`：`workspace/skills/<name>/SKILL.md`
- Context：`AgentRulesAndSkills`
- **不**创建 `EvolutionProposal`

## 3. 数据流

```text
trigger_autodream
  → .agent-diva/autodream/**
  → Worker → CandidateGate
  → .laputa/proposals/**
  → governance.sqlite3 decide/receipt
  → apply sections / typed memory + changelog
  → EvolutionView inbox
  → ApprovalCenter 可 deep-link
```

Skill 文件树旁路：`skills/**` 不经上述链。

## 4. 持久化（Evolution 相关）

| 路径 | 内容 |
| --- | --- |
| `.agent-diva/autodream/**` | lock/checkpoint/events/runs |
| `.laputa/proposals/**` | 提案 |
| `.laputa/governance.sqlite3` | 治理账本映射 |
| `.laputa/changelog|audit|rollback/**` | 审计 |
| `.laputa/candidate-suppression.json` | 抑制 |
| `.laputa/recall-feedback.json` | 召回反馈 |
| `.laputa/reports/**` | 日/周/月报告 |
| `skills/**/SKILL.md` | Skill 权威（非 .laputa） |

## 5. 与决策冲突面

| 决策 | 冲突 |
| --- | --- |
| D1 Memory 不审批 | MemoryPatch + governed apply + inbox |
| D2 退役 AutoDream–Evolution | 整 crate + GUI + e2e 仍主路径 |
| D3 只管 Skill 可管理性 | 无 Skill 列表/版本/验证；Skills 在设置页 |
| D3 不承载 Persona | Identity* 类型与 workspace 投影 |
| Approval 仅危险工具 | ledger 仍服务 memory/persona proposals |

## 6. 本切片未覆盖（完整 R0 债）

Persona Diff/历史全量；BML 工具面；STM/checkpoint；memory_md 删除；cron 触发细节；生产 profile 抽样；GA 远程 tip 文件级 diff。
