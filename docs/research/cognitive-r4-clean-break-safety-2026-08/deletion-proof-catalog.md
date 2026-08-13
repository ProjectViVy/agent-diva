# R4 零残留证明目录

- 状态：`Research Draft / Catalog Only`
- 日期：2026-08-13
- 性质：实施后必须扫到「零」的符号、路径、路由、GUI、Prompt、测试；**现在不改扫描脚本**
- 权威清单：R0 [`dependency-and-data-inventory.md`](../cognitive-r0-current-state-2026-08/dependency-and-data-inventory.md) §4
- 现成门禁 `just laputa-clean-break-check` **不够**本 EPIC：它只拦 `mentle`/`memtle`
  与已删 migration 模块回潮（`scripts/ci/check_laputa_clean_break.py`）

## 1. 证明分层

| 层 | 何时跑 | 谁写具体正则 | 本包 |
| --- | --- | --- | --- |
| A. 仓库静态扫描 | 每个删除切片 + `just ci` | D4 扩现有 Python 门或新 recipe | 列目标与误报 |
| B. 单元/集成负向测试 | 同切片 | 各 crate | 列必须翻掉的旧锁 |
| C. 运行时纵向 | Architecture Gate 后桌面 | 验收矩阵 | 列观察点 |
| D. 用户工作区只读审计 | 发布前操作员 | 手顺 | 见影响报告 §5 |

A 层失败 = 切片不得合并。B 层旧测试若仍要求 JSON/`memory_md`/提案保存，必须在
**同一切片**改掉或删除，不能留着当兼容。

## 2. 扫描族（A）

下列每一族都要有「应命中 / 禁止误伤」。

### 2.1 `memory_md` 文件权威

应消失（生产路径）：

- `LaputaSectionName::MemoryMd`、`as_str() = "memory_md"`
- `ProposalType::MemoryPatch` 以 MemoryMd 为 target
- `POST /api/laputa/section/memory_md/write`
- GUI `SectionGroupList` `long_term`、i18n `laputa.sections.memory_md`
- 路径 `.laputa/sections/memory_md.json`、`MEMORY.md` seed、`MemoryManager` 默认

禁止误伤：

- BML `MemoryRecordKind::LongTerm`
- Memory 页 kind 筛选 `long_term`
- `canonical_checkpoint_v1`

### 2.2 Persona JSON 权威

应消失：

- `LaputaSection.content: serde_json::Value` 作为人格正文
- `section_content_type` → `"json"`
- `SectionEditor` `JSON.parse` / `formatJson` / `jsonError`
- `PersonaMemoryView.formatContent` → `JSON.stringify`
- `create_user_edit_proposal` 四人称臂
- `ProposalType::{IdentityPatch,RelationshipUpdate,CommitmentSet,LearningNote}` 作为人格载体
- `laputa_propose_section_write` JSON 合同
- `persona_retire` CLI 与 `metadata.migration`
- `FIRST_RUN_ONBOARDING_BLOCK`、`initialize_sections` 的 `null` 种子
- Frozen Core `serde_json::to_string` 紧凑 JSON 投影

禁止误伤：

- HTTP/Tauri **信封** JSON（section 名、revision、错误码）
- Identity / Relationship / Commitment / Preferences **概念名**
- `content_version` 算法若 D1 仍用 SHA（可 Renames）

### 2.3 Persona/Memory 治理链

应消失（对这些域）：

- `MemoryGovernanceCoordinator`、Approval `domain=memory`、`MemoryApply`
- `governance.sqlite3` / `memory_proposal_governance`
- GUI `PersonaLifecyclePanel`、Persona/Memory 用的 `ProposalInbox`
- Chat `openEvolutionProposal`、Memory 页 `open-approval`
- `ProposalType::SopCreate` → Identity

必须留下：

- `/api/approvals` 的 command/plan
- M3 危险工具
- SkillsLoader + `skills/*/SKILL.md`

### 2.4 旧 Evolution / AutoDream 主链

应消失或不再从 Evolution 页加载（D3 定「报告是否独立」）：

- Evolution 文案捆 personality/memory/SOP/skill
- AutoDream → `EvolutionProposal` 默认 `MemoryPatch`
- 若 D3 删主链：`/api/autodream/runs/**`、`.agent-diva/autodream/` 运行时依赖

DECIDE（D3/D4）：`reports/`、Notebook daily/weekly/monthly section。

### 2.5 Prompt / 初始化

应消失：

- Prompt 驱动 `ask_user` + `laputa_propose_section_write` 当人格初始化
- `WELCOME_STORAGE_KEY` **作为人格完成标记**（密钥向导可留）
- 完整 WORLD / MEMRULES / 退役人格文件注入（已有 `context_plane_invariants`）

新证明（D1 后加）：

- Frozen Core 投影是 Markdown 字符串，不是 `{`
- 无五文件预种子；缺席检测成立

### 2.6 事件与遗留前端

应消失：

- `LaputaEventKind` 作为 Persona/Memory 总线（随旧治理）
- `useEventStream.ts`（未接入，R0 DELETE）

留下：`AgentEvent` 聊天 SSE、统一 approvals 事件（无 memory 域）。

## 3. 必须改写的旧测试锁（B）

删除切片若留下这些断言，等于禁止 clean break：

| 测试 | 锁住的旧合同 |
| --- | --- |
| `frozen_core.rs` seed/`null`/紧凑 JSON | JSON 权威 |
| `cognitive/sections.rs` 预种子 | P10 反面 |
| `create_user_edit_proposal.rs` / `propose_section_write.rs` | 提案式保存 |
| `agent_loop.rs` First-Run Onboarding | Prompt 初始化 |
| `SectionEditor.spec.ts` 非法 JSON 禁保存 | JSON 门 |
| `PersonaMemoryView.test.ts` `null` → `'{}'` | 对象编辑器 |
| `SectionGroupList.test.ts` `memory_md` | 左栏 LTM |
| autodream `MemoryMd` fixtures | 旧输入 |
| manager ` /section/memory_md/write` | JSON 写路由 |

R0 失败基线测试（`errorMessage` 禁 `[object Object]`）**先留着**当证伪，直到
Markdown 全链路测试取代它们。

## 4. 运行时观察点（C）

发布候选必须人工或脚本看到：

1. 打开 Persona：中央是 Markdown，不可能插值出 `[object Object]`
2. Evolution **不**加载 Persona/Memory/旧 AutoDream proposal 收件箱
3. Memory 增删改查、Persona 直存、STM（若已有）**不**查询 Governance Ledger
4. Chat Approval Center 仍能走完一条危险工具授权
5. 新工作区：五权威全缺才出现首次引导；`LaputaStorage::open` 不再写 `null` JSON /
   `# WORLD\n` 冒充 ready
6. 旧工作区副本：新二进制不转换、不提示「已导入」；用户只能看见 KEEP 面（BML/skills）

## 5. 建议的机器检查形状（非实施）

D4 可扩 `scripts/ci/check_laputa_clean_break.py` 或加 `just cognitive-clean-break-check`：

- 正则分族，带 allowlist 文件（本 catalog、R0 inventory、归档 docs）
- 生产扫描根与现脚本相同 crate 集，**排除** `docs/dev/archive`、`docs/research` 里的
  历史证据（否则研究包自己会红）
- 自测：临时文件插入 `MemoryMd` / `formatJson` / `domain=memory` 必须失败

本包不提交该脚本。现 `laputa-clean-break-check` 继续守 mentle 门。

## 6. 误报与 DECIDE 暂缓

扫描时不要把 DECIDE 当已删：

| 符号 | 为何暂缓 |
| --- | --- |
| BML `MemoryRecordKind::Identity` 等 | D0 |
| `WorldStore::project` / WorldGovernance | D1 |
| MEMRULES 左栏 | D0/D1 |
| `changelog/` `audit/` `rollback/` | D1/D4 |
| AutoDream `reports/` / Notebook | D3 |
| `/api/command-approvals` | 运行时，非本 EPIC 主链 |

这些在 D 拍板前出现在代码里**不是** R4 失败。

## 7. 与保护分支的关系

零残留证明跑在**新树**上。保护分支故意保留全部旧符号，不得拿保护树跑本目录的
A 层扫描当发布门禁。
