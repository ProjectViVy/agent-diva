---
story_key: 1-9-force-light-path-fr21
story_id: "1.9"
epic: 1
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/planning-artifacts/implementation-readiness-report-2026-03-30.md
---

# Story 1.9：强制轻量路径与 FR21 冻结文档

Status: done

## Story

As a **维护者**,  
I want **在实现说明中冻结「强制轻量」与「大脑皮层 OFF」是合并还是独立策略**,  
So that **FR21 可测且与 FR3 不打架**。

## Acceptance Criteria

1. **Given** ADR 或 `docs` 片段 **二选一并写明**（合并 vs 独立配置位）  
   **When** 策略为 **ForceLight**（命名以实现为准）  
   **Then** 编排 **不** 为多视角对弈链预留额外模型回合，直至用户显式升级本次任务

2. **And** Story **1.4** / **4.3** **交叉引用** 该冻结文档（链接路径或相对仓库路径写死）

3. **And** 至少 **一条** 自动化测试 **或** 可执行清单项验证 **ForceLight**（当实现选型为 **独立于** `CortexState::Off` 时必选；若已合并为同一语义，则清单/测试须明确覆盖「合并语义」下的关皮层 + 强制轻量等价行为）

## Tasks / Subtasks

- [x] **首版实现 ADR 落文**（AC: #1）  
  - [x] 在仓库约定位置（如 `agent-diva-swarm/docs/`、`architecture.md` 增补段落，或独立 `docs/adr/xxx-fr21-force-light.md`）**写死二选一**：  
    - **选项 A — 合并：** `ForceLight` 与 **大脑皮层 OFF** 为同一用户可感知语义（关皮层即强制轻量；文案与 UX 与 PRD/UX 规格 D.3 对齐）  
    - **选项 B — 独立：** `ForceLight` 为 **独立** 配置位/会话标志；皮层 **开** 时仍可强制轻量；须在设置或等价入口 **单点命名** 且 **可键盘到达**（UX 规格）  
  - [x] 与 **`CortexState::Off`（FR3）** 的关系在 ADR 中 **交叉说明**，避免与 Story **1.4** 的「关模式简化语义」矛盾

- [x] **与 architecture.md ADR-E 对齐**（AC: #1）  
  - [x] 引用或摘录 **FR21 / ForceLight** 与 **CortexState::Off** 的已选方案，保证与 `architecture.md` — *ADR-E* 表述一致或显式「本 ADR 取代/细化」段落

- [x] **交叉引用 Story 1.4 与 4.3**（AC: #2）  
  - [x] **Story 1.4**（「关大脑皮层」简化模式 — 无头测试）：在冻结文档中说明 **1.4 的测试语义** 如何依赖本 FR21 选型（合并则关模式测试覆盖 ForceLight；独立则须 **额外** 测 ForceLight 分支）  
  - [x] **Story 4.3**（「关大脑皮层」语义说明文档 — FR17）：在 4.3 目标文档中 **反向链接** 本冻结文档；本故事完成时须在 4.3 草稿或正文中加入 **「见 FR21 冻结：…」** 段落（路径固定）

- [x] **测试或清单**（AC: #3）  
  - [x] **若独立 ForceLight：** 至少一条 **无 GUI** 测试：`ForceLight == true` 且皮层 **开** → 编排仍 **不** 进入 FullSwarm 对弈链（与 1.7 分层一致）  
  - [x] **若合并：** 清单或测试断言：**皮层 OFF** 路径与「不预留多视角对弈额外回合」一致，且与 PRD FR21 强制期间语义一致  
  - [x] 将测试文件路径或清单 Markdown 路径写入冻结文档 **验收附录**

- [x] **验证**  
  - [x] 评审：PM/架构可仅凭冻结文档 + 链接判断「合并 vs 独立」且无 FR3/FR21 歧义  
  - [x] `implementation-readiness-report` 类门禁：FR21 二选一已 **写死**（与 IR 报告「实现冻结」行一致）

## Dev Notes

### Epic 1 上下文

本故事不替代 **1.7**（默认轻量路由）或 **1.8**（收敛/触顶）；专责 **显式强制轻量（FR21）** 与 **皮层 OFF（FR3）** 的 **产品语义与实现选型冻结**，使后续实现可测、可文档化。

### 架构合规（必须遵守）

| 主题 | 要求 | 来源 |
|------|------|------|
| FR21 二选一 | 合并 **或** 独立；**禁止** 实现期长期模糊两种语义 | `epics.md` Story 1.9；`implementation-readiness-report-2026-03-30.md` |
| ADR-E | **FR21 / ForceLight** 与 **CortexState::Off** 在 ADR 中交叉说明 | `architecture.md` — ADR-E |
| 可测性 | 无 GUI 覆盖 **ForceLight**（若独立于 OFF）；扩展 FR12 方向 | `architecture.md` — ADR-E 可测性 |

### 与 Story 1.4、4.3 的关系（交叉引用矩阵）

| 故事 | 角色 | 本故事交付物中的动作 |
|------|------|----------------------|
| **1.4** | 关皮层行为与无头测试 | 冻结文档定义 1.4 测试应断言的「简化/强制轻量」边界；1.4 实现故事须 **链接** 本文档 |
| **4.3** | 用户向「关大脑皮层」说明（FR17） | 4.3 正文 **引用** FR21 冻结路径；若选合并，4.3 须写清「关 = 无多代理对弈链」与 UX-DR 对齐 |

### PRD / UX 摘要

- **PRD FR21：** 用户或配置可 **显式强制轻量路径**；可与皮层 OFF **合并** 或 **独立** —— **须在链接文档中冻结**；强制期间 **不** 为多视角对弈链预留额外模型回合，除非用户 **随后** 显式升级。  
- **UX 规格 D.3：** 若合并，关皮层文案与空状态须一致传达「无多代理对弈链」；若独立，设置中单点命名并可键盘到达。

### 依赖与顺序

- **建议前置：** Story **1.2–1.3**（状态真相源）、**1.7**（分层枚举/路由存在后，ForceLight 才有挂载点）。  
- **可并行起草：** 与 **1.8** 并行，但冻结文档须在 **全量蜂群路径联调前** 定稿，避免返工。

### 参考路径（仓库内）

- `_bmad-output/planning-artifacts/epics.md` — Story 1.9、1.4、4.3  
- `_bmad-output/planning-artifacts/architecture.md` — ADR-E、CortexState  
- `_bmad-output/planning-artifacts/prd.md` — FR21、FR3  
- `_bmad-output/planning-artifacts/ux-design-specification.md` — D. 轻量路径与收敛  

---

## 验收清单（实现完成后勾选）

- [x] ADR/文档中 **明确** 选项 A（合并）或 选项 B（独立），无「待议」  
- [x] 与 `CortexState::Off` / FR3 无冲突表述  
- [x] Story **1.4** 相关测试说明或代码注释 **链接** 本冻结文档路径  
- [x] Story **4.3** 目标文档 **链接** 本冻结文档路径  
- [x] **至少一条** 测试 **或** 清单项满足 AC #3（独立则必测 ForceLight）

---

## Dev Agent Record

### Implementation Plan

- 确认既有冻结文档 `agent-diva/agent-diva-swarm/docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`（选项 A 合并）满足 AC #1；增补与 `architecture.md` ADR-E 的 **细化/非取代** 说明。
- 在 `architecture.md` ADR-E **FR21** 条目中写死首版路径与选型，满足与规划文档一致。
- 新增无 GUI 测试 `fr21_merge_off_path_no_full_swarm_extra_rounds`，并在 ADR §4、`CORTEX_OFF_SIMPLIFIED_MODE.md` 对照表登记。
- 更新 IR 报告「实现冻结」行与 Concern 表；在 Story **1.4** 工件中增加指向本 ADR 的固定相对路径。

### Debug Log

- （无）

### Completion Notes

- `cargo test -p agent-diva-swarm` 全量通过（46 tests）。  
- FR21 合并语义由具名测试 + ADR 验收附录双重可追溯；4.3 维护者指南中「见 FR21 冻结」段已在前期落地，本故事未改 4.3 正文。

## File List

- `agent-diva/agent-diva-swarm/docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`（修订）  
- `agent-diva/agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md`（测试对照表增补）  
- `agent-diva/agent-diva-swarm/src/minimal_turn.rs`（FR21 合并语义测试）  
- `_bmad-output/planning-artifacts/architecture.md`（ADR-E FR21 首版路径与选型）  
- `_bmad-output/planning-artifacts/implementation-readiness-report-2026-03-30.md`（实现冻结行 + Concern）  
- `_bmad-output/implementation-artifacts/1-4-cortex-off-headless-tests.md`（交叉引用 ADR 路径）  
- `_bmad-output/implementation-artifacts/1-9-force-light-path-fr21.md`（本文件）

## Change Log

- 2026-03-31：Story 1.9 完成 — ADR-E 与 swarm ADR 双向对齐、IR 门禁闭合、FR21 合并语义具名测试与 1.4 工件互链；故事状态 → review。
