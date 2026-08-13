# R4 Clean-break 用户数据影响

- 状态：`Research Draft / Source-backed`
- 日期：2026-08-13
- 性质：删除旧链路后用户会失去什么；**不设计**导入、双读、双写或 runtime fallback
- 输入：R0 [`dependency-and-data-inventory.md`](../cognitive-r0-current-state-2026-08/dependency-and-data-inventory.md)、
  [`legacy-failure-baseline.md`](../cognitive-r0-current-state-2026-08/legacy-failure-baseline.md)；
  R1 Evolution 错位；R3 Persona JSON ≠ 文档历史
- KEEP / DELETE / DECIDE 仍是研究标记，不是 D4 删除切片

## 1. 一句话影响

破坏性实施之后，新 runtime **不会**再读旧 Persona JSON、`memory_md`、persona-retire
源文件、Persona/Memory 的 `EvolutionProposal` 和 Memory 域治理账本。用户若只靠这些
旧面保存内容，那些字节会留在磁盘上变成孤儿，或被发布说明要求自行备份后丢弃。
**BML `memory.sqlite3` 与 `skills/*/SKILL.md` 不在删除面**（R0 KEEP）。

本会话**没有**抽样任何生产 profile 的真实体积。下列影响按代码路径推导。
操作员在 D4 之前应按 [`protection-branch-protocol.md`](./protection-branch-protocol.md)
§6 在自己的工作区跑一次目录清单。

## 2. 按数据族的损失面

| 数据族 | 今天落在哪 | 删/停读之后用户失去什么 | 仍在的东西 | 标签 |
| --- | --- | --- | --- | --- |
| 四份人格 JSON | `.laputa/sections/{identity,relationship,commitment,preferences}.json`，启动常为 `null` | 旧 GUI 能看见的 JSON 正文；Legacy apply 写进文件的内容 | Typed apply 进了 BML `Identity` 等 kind 的行（D0 双权威，**不是**新 Persona Markdown） | 源码事实 R0/R3 |
| 人格「历史」 | `changelog/<id>.json` + HistoryModal 50 条 | 提案 apply 审计、30 天 rollback 能力 | 不是产品要的永久 Markdown 轨迹；新历史必须另建 | 源码事实 R3 |
| 待审人格/记忆提案 | `.laputa/proposals/*.json` + `governance.sqlite3` 映射 + Approval `domain=memory` | 未 apply 的编辑；右栏/Evolution/Approval 里的 Memory 审批单 | 已 `memory_add` 的 BML LongTerm | 源码事实 R0 |
| `memory_md` | `sections/memory_md.json`（不预种子）；`memory_update` / Notebook / AutoDream 默认写这里 | **只走了 MemoryPatch、从未进 BML 的长期编辑** | `memory_add` 直写的 BML LongTerm | 源码事实 R0 §4.1 / 写入者表 |
| 根/工作区 Markdown 人格与 MEMORY | `SOUL.md` `IDENTITY.md` `USER.md` `BOOTSTRAP.md` `MEMORY.md` `memory/MEMORY.md` | 若仍当权威编辑，新 runtime 不再探测 | `.laputa/legacy/` 若曾 archive | 源码事实 R3 persona-retire |
| AutoDream 主链 | `.agent-diva/autodream/**`、`/api/autodream/runs`、Evolution runs 页 | 运行记录、live-text、未落地的候选 | 已 apply 进 BML 的内容；`skills/` 不被 AutoDream 当主产物（R1） | 源码事实 R1 |
| WORLD 种子与队列 | 种子 `# WORLD\n`；`world-proposals.json` pending | 预种子本身无 claim；**未 apply 的 consolidation pending**（今天也无生产 apply） | 人类已写进 `WORLD.MD` 的 claims | 源码事实 R3 |
| Memory 域治理 | `governance.sqlite3`；`governance.db` 里 capability=`MemoryApply` | 旧审批恢复、`needs_attention` 单 | `governance.db` 的 command/plan（KEEP，必须留下） | 源码事实 R0 |
| 三症状修补 | `d6f82ea3` 等 GUI 字符串化 | 失去的是旧链补丁，不是数据 | 失败基线仍作验收证伪 | 提交事实 R0 |

不要误删（R0 已写）：BML `MemoryRecordKind::LongTerm`、GUI Memory 页 `long_term` 筛选、
`canonical_checkpoint_v1`、SkillsLoader、M3 Approval Center。

## 3. 用户可感知的「我的东西呢」

推断，按今天默认 Typed 生产路径：

1. **聊天里 `memory_add` 的记忆大概率还在。** 它们在 `memory.sqlite3`，不经过
   `memory_md`。
2. **Persona 页上看到的 JSON 多半不是权威。** 保存只建提案；Typed apply 写 BML
   不回写 JSON。删 JSON 文件，GUI 旧页会空，但 BML Identity 行可能仍在——那也
   **不会**自动变成新 Markdown 人格（D0 必须消解，禁止当导入器）。
3. **`memory_update` / Notebook / AutoDream 的补丁可能真丢。** 这些默认进
   `MemoryPatch` → `memory_md`，R0 写明不写 BML。这是最需要一次性备份的一族。
4. **未批准的提案会消失在产品面。** 文件可仍在磁盘，新 runtime 不再列出。
5. **WORLD 人类正文应保留概念。** 删的是预种子合同和独立治理队列，不是「用户写过的
   环境说明」这一产品对象。物理路径 D1 Hold。
6. **Skill 文件树应留下。** Evolution 要重做管理面，不是删 `SKILL.md`。

## 4. 一次性人工备份（允许的唯一保护）

产品：不提供自动导入、启动迁移、双读、双写、兼容 DTO、隐藏恢复。
保护性分支不是 fallback。

建议（非批准）发布说明只给**操作员手顺**，由 D4 定最终措辞：

```text
1. 停掉 GUI / gateway，确认无写入。
2. 整目录复制工作区（至少 .laputa/、.agent-diva/、根 SOUL/IDENTITY/USER/MEMORY.md、
   memory/、skills/）。
3. 另存一份 git 保护分支 tip 的 zip（见协议文）。
4. 用只读方式打开副本核对：BML 能否用现 Memory 页列出；Persona JSON 是否几乎全 null；
   memory_md.json 是否有用户正文。
5. 需要保留的正文，由用户在新工作区**重新键入或粘贴**到新权威。产品不转换。
```

禁止在手顺里加入：转换脚本、启动时探测旧文件、把 BML Identity 行投影成新
Persona Markdown、「找不到就读 legacy」。

`agent-diva-migration` 现有 MEMORY.md → LongTerm 离线导入（R0）列入 **DELETE
兼容链**；不得当作本轮发布工具。

## 5. 未抽样项（操作员清单）

下列体积/内容必须在真实桌面量一次，本包不编数字：

| 路径 | 为什么要看 |
| --- | --- |
| `.laputa/sections/*.json` | 是否几乎全 `null`，还是有 Legacy apply 正文 |
| `.laputa/proposals/` 文件数 | 未完成审批的损失面 |
| `.laputa/changelog/` | 治理审计体积；不是新历史 |
| `.laputa/memory.sqlite3` | KEEP；备份完整性 |
| `.laputa/governance.db` vs `governance.sqlite3` | 危险工具账本 vs Memory 映射 |
| `.laputa/cognitive/WORLD.MD` | 是否已有人类 claims |
| `.agent-diva/autodream/runs/` | 运行史是否用户在意 |
| 工作区根 `SOUL.md` 等 | 是否仍被当权威编辑 |

## 6. 对发布与验收的约束

- 破坏性发布必须写明：旧 Persona JSON / `memory_md` / 未 apply 提案 **不会**出现在
  新 GUI；BML 记忆仍在 Memory 工作区。
- 失败回滚 = 回到保护性分支检出的旧二进制 + 备份目录，**不是**新二进制读旧格式。
- 三条旧桌面症状（`[object Object]`、Evolution 挤作一团、`governance ledger failed`）
  不再hotfix；最终验收按 R0 失败基线证伪。
- AutoDream 报告 / Notebook section 是否独立于 Evolution：仍 DECIDE，交给 D3。
  R4 只要求：即使报告留下，也不得再创建 Persona/Memory Proposal。

## 7. 开放问题（不在本包拍板）

| 问题 | 交给 |
| --- | --- |
| BML Identity 行删、留、还是仅只读审计 | D0 |
| WORLD 物理路径与 `world-proposals.json` 去留 | D1 |
| changelog/audit/rollback 目录是删还是冻结 | D1/D4 |
| AutoDream 报告目录 | D3/D4 |
| 精确删除提交切片与顺序 | D4 |
