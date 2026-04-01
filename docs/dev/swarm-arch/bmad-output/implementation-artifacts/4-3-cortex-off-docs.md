---
story_key: 4-3-cortex-off-docs
story_id: "4.3"
epic: 4
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/implementation-artifacts/1-4-cortex-off-headless-tests.md
  - _bmad-output/implementation-artifacts/1-9-force-light-path-fr21.md
---

# Story 4.3：「关大脑皮层」语义说明文档（FR17）

Status: done

## 依赖与顺序

| 故事 | 角色（本 story 中） |
|------|---------------------|
| **1.4** | 提供 **关皮层简化模式** 的 **实现登记文档** 与 **无头测试语义**；本 story 须 **逐条对齐** 并 **交叉引用** 其「测试对照表」与 AC。 |
| **1.9**（建议前置或并行定稿） | **FR21**「强制轻量」与皮层 OFF **合并/独立** 的 **冻结说明**；本故事正文须含固定路径的 **「见 FR21 冻结：…」** 段落，避免与维护者向说明矛盾。 |

**说明：** Epic 4 内仅要求序号更小的故事为依赖时，**4.1、4.2** 与本故事无硬阻塞；**FR17** 的验收核心是 **仓库内可读文档 + 与 1.4 测试语义可追溯**。

## Story

作为一名 **维护者**，  
我希望 **仓库内有一份与 MVP 对齐的说明**，  
以便 **FR17 满足**：能独立理解 **「关大脑皮层」** 的语义、行走路径、限制及与 **「开」** 模式的差异。

## Acceptance Criteria（摘自 epics，须全文满足）

1. **Given** `docs` 或 crate 文档目录（与下方 **MVP 文档路径** 一致）  
   **When** 阅读该文档  
   **Then** 明确 **关** 模式下的 **行走路径**、**限制** 及与 **开** 模式的 **差异**  
2. **And** 与 Story **1.4** 测试语义 **交叉引用**（链接到实现登记文档章节、测试模块/函数名或本仓库 story 工件路径）

## FR17 与相邻需求的分工

| 需求 | 本故事（4.3）侧重 | Story 1.4 侧重 |
|------|-------------------|----------------|
| **FR17** | 维护者向、**与 MVP 产品路径一致** 的 **可读说明**（路径、边界、开/关差异） | **FR3 / FR12**：可执行语义登记 + **headless** 断言与 **doc-ref** 追溯 |
| **FR21** | 在正文中 **引用 1.9 冻结文档路径**，写清「关」与 **强制轻量** 在用户感知上是否等价（随 1.9 选型） | 文档中单列「与 FR21 的边界」；测试如何覆盖由 1.9 矩阵定义 |

与 `prd.md` **FR3、FR12、FR17** 一致：**FR17** 偏 **维护者说明**；**1.4** 偏 **实现者与无头测试**（见 `1-4-cortex-off-headless-tests.md` 内说明）。

## MVP 文档路径（与规划对齐）

以下路径为 **MVP 验收落点**；若实现阶段改名，须在本 story 完工时的 **File List** 中登记 **实际路径** 并 **更新此处链接**。

1. **主文档（FR17，推荐）**  
   - 目录：`agent-diva-swarm/docs/`（与 Story **1.1** workspace 成员及 **1.4** 建议落点一致）。  
   - 文件名以实现为准；须 **独立成文** 或作为该目录下 **单一维护者入口** 的置顶章节（避免 FR17 内容仅散落在 issue/PR 描述中）。

2. **与 1.4 实现文档的关系**  
   - Story **1.4** 登记的 **《简化模式语义》**（例如 `CORTEX_OFF_SIMPLIFIED_MODE.md` 或 File List 中的等价路径）为 **技术真值**；本故事文档须 **显式链接** 该文件，并 **摘要** 维护者需知的结论（禁止与 1.4 登记条目矛盾）。

3. **与 UX / GUI 仓库的衔接（可选但推荐）**  
   - `ux-design-specification.md` 要求：在实现 PR 或 `agent-diva-gui` **README 附录** 中维护与蜂群相关的维护者说明时，**附录须链接** 上述 swarm `docs` 中的 FR17 主文档，形成 **MVP 双入口**（GUI 贡献者 ↔ 语义真值）。

## 与 Story 1.4 测试语义的交叉引用（必做）

在 FR17 主文档中须包含 **「测试与追溯」** 小节，至少：

- **链接** 实现工件：`_bmad-output/implementation-artifacts/1-4-cortex-off-headless-tests.md`（便于规划/评审检索）。  
- **链接** 1.4 交付的 **实现登记文档** 路径（与 1.4 File List 一致）。  
- **摘录或指向** 1.4 文档中的 **「测试对照表」**：`文档章节 / 断言摘要 / 测试模块或函数名`；并说明 **维护者说明** 中的每一条 **可验证主张** 能在该表或 1.4 AC 中找到对应项。  
- **开/关负向用例**：注明 1.4 **AC #4** 要求至少一条用例在 **错误分支** 下失败且日志可检索「大脑皮层」「开」「关」等关键词 —— FR17 文档应用 **非实现者语言** 概括其 **意图**（例如：确保 CI 能区分分支错误与数据错误），并指向 1.4 细节。

## FR21 冻结段落（必做）

在正文中加入固定小节标题，例如：**「与强制轻量路径（FR21）的关系」**，内容为：

- **见 FR21 冻结：** `<由 Story 1.9 写死的相对仓库路径>`  
- 用 **一两段话** 说明：当前产品语义下 **「关大脑皮层」** 是否与 **ForceLight** 合并，以及维护者阅读 **1.4 测试** 时应如何理解 **关路径 vs 强制轻量**（与 `1-9-force-light-path-fr21.md` 交叉引用矩阵一致）。

在 **1.9 未完成** 前，可保留 **占位路径** 与 **「待 1.9 定稿后替换」** 标注，但 **本 story 进入 review 前** 须已替换为 **真实冻结路径**（与 1.9 AC #2 闭环）。

## Tasks / Subtasks

- [x] **选定 FR17 主文档文件名并落盘**于 `agent-diva-swarm/docs/`（或与 File List 一致的 MVP 路径）（AC: Given/Then）  
- [x] **撰写开/关差异**：行走路径、限制、与「开」模式对比表或分节说明（AC: Then）  
- [x] **交叉引用 1.4**：实现登记文档 + 测试对照表 + 1.4 story 工件链接（AC: And）  
- [x] **插入 FR21 冻结引用段落**，与 `1-9-force-light-path-fr21.md` 交付物路径一致（见上节）  
- [x] **（推荐）** 在 `agent-diva-gui` README 附录增加 **指向 FR17 主文档** 的链接（UX 规格；与 MVP 双入口一致）  
- [x] **验证**：非实现读者仅读 FR17 文档 + 链接即可理解「关大脑皮层」边界；与 `prd.md` FR17、`epics.md` Story 4.3 无矛盾  

### Review Findings

（Code review 2026-03-31，story `4-3-cortex-off-docs`，diff 来源：`agent-diva` 工作区未提交变更 + 未跟踪文件。）

- [x] [Review][Patch] FR17 主文档 §2 对比表未覆盖「显式全蜂群 + 皮层关」限制（`CORTEX_OFF_SIMPLIFIED_MODE.md` §2 已登记 `explicit_full_swarm_suppressed_by_cortex_off`；维护者向摘要应有一句或一行表项，避免与 1.4 真值漂移） [`CORTEX_OFF_FR17_MAINTAINER_GUIDE.md`] — 已补表行
- [x] [Review][Patch] `ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md` §4 验收附录仍写 `cortex_off::*`，与登记文档测试对照表中的 `minimal_turn::cortex_off_tests::*` 不一致，检索/复制粘贴会误导 [`ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`] — 已改为与对照表一致的模块路径

## Dev Notes

### Epic 4 上下文

Epic 4 目标含 **单一 Person 叙事**、**skills/能力包与校验**、**文档与诊断（FR17–FR18）**。本故事仅覆盖 **FR17**；**FR18** 由 Story **4.4** 负责。

### References

- `_bmad-output/planning-artifacts/epics.md` — Epic 4，Story 4.3；FR Coverage Map  
- `_bmad-output/planning-artifacts/prd.md` — **FR17**  
- `_bmad-output/planning-artifacts/architecture.md` — 文档与诊断（FR17–FR18）总述  
- `_bmad-output/planning-artifacts/ux-design-specification.md` — 维护者文档与 FR17、附录约定  
- `_bmad-output/implementation-artifacts/1-4-cortex-off-headless-tests.md` — Story 1.4，测试语义与 AC  
- `_bmad-output/implementation-artifacts/1-9-force-light-path-fr21.md` — FR21 冻结与 4.3 反向链接约定  

## Dev Agent Record

### Agent Model Used

Cursor / Composer（bmad-dev-story）

### Debug Log References

（无）

### Implementation Plan

- 新增 FR17 主文档 `CORTEX_OFF_FR17_MAINTAINER_GUIDE.md`（产品语言 + 开/关表 +「测试与追溯」+ FR21 小节）。
- 新增 FR21 冻结 ADR `ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`（选项 A 合并选型，满足进入 review 所需的**真实冻结路径**；与 1.4 登记文档、1.9 矩阵互链）。
- `CORTEX_OFF_SIMPLIFIED_MODE.md` §3 增加指向 ADR/FR17 的交叉引用。
- `agent-diva-gui/README.md` 附录双入口链接至上述文档。
- 修复 `neuro_overview.rs` 对 `SwarmRunFinished` / `SwarmRunCapped` 的匹配与 `detail: Option<String>`，保证 `cargo test -p agent-diva-swarm` / clippy 通过。

### Completion Notes List

- FR17 验收落点：**`agent-diva/agent-diva-swarm/docs/CORTEX_OFF_FR17_MAINTAINER_GUIDE.md`**。
- FR21 冻结落点：**`agent-diva/agent-diva-swarm/docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`**（与 Story 1.9 AC #2 约定路径一致，供 1.9 后续迭代同一文件而非占位）。
- 已运行：`cargo test -p agent-diva-swarm`（31 通过）、`cargo clippy -p agent-diva-swarm -- -D warnings`。

### File List

- `agent-diva/agent-diva-swarm/docs/CORTEX_OFF_FR17_MAINTAINER_GUIDE.md`（新增，FR17 主文档）
- `agent-diva/agent-diva-swarm/docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`（新增，FR21 冻结）
- `agent-diva/agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md`（§3 交叉引用 ADR/FR17）
- `agent-diva/agent-diva-gui/README.md`（附录：蜂群 FR17 链接）
- `agent-diva/agent-diva-swarm/src/neuro_overview.rs`（`ProcessEventNameV0` 新变体 + `detail` 类型修复）
- `_bmad-output/implementation-artifacts/sprint-status.yaml`（4-3 → review）
- `_bmad-output/implementation-artifacts/4-3-cortex-off-docs.md`（本文件）

### Change Log

- 2026-03-31：完成 Story 4.3 文档与交叉引用；新增 FR21 ADR；GUI README 附录；修复 `neuro_overview` 编译；sprint 4-3 → review。

---

_Context: BMad story context — 对齐 `epics.md` Story 4.3、`prd.md` FR17；与 Story 1.4 / 1.9 交叉引用；简体中文。_
