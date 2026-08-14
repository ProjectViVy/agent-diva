# D4 — Clean-break 接口、删除、切片与验收

- 状态：`Approved / Implementation Pending`
- 日期：2026-08-15
- 性质：把已批准 D1–D3 收成可实施的切片、删除证明、发布与验收。通过本包 + 你叫切，才是 Architecture Gate。
- **不改**生产代码。**不建**保护分支（你说切再切）。
- 用户批准：`2026-08-15`（对话「点头」）。同日 STM Recap 修订不改本包切片顺序。

不得重开：P/S/D6/D7、D1–D3 合同、无迁移（没了就没了）。

---

## 0. 门禁

```text
D1/D2/D3 已批准
    → 本包评审
    → 你说「切」建保护分支
    → Architecture Gate
    → 按切片改生产（每片独立验证、独立 commit）
```

D0 图已由后续批准实际生效（P22 / S1 / D7 / D1–D3）。不再单独拦门。

禁止：双读、启动导入、runtime fallback、把保护分支当第二套实现。

---

## 1. 实施依赖与切片

原则：每片可独立回滚；先停旧权威写入，再立新权威，再拆旧读，最后扫门。

| 片 | 名 | 做什么 | 依赖 | 验证 |
| --- | --- | --- | --- | --- |
| **P0** | 保护分支 | 见 §4。只记录、不在本包执行 | 你说切 | `git branch` 存在 |
| **S1** | 停种子 | 删 `null` JSON / `# WORLD` 预种子。**不**切换 P10 三态、不接引导 API | P0 | `open` 不再写 `sections/*.json` null、不再写 `# WORLD\n` |
| **S2** | Persona 家 | `{config_dir}/persona/` + `/api/persona` + FC 读 MD + GUI + `persona_request` 工具。该域生产读写只打新家 | S1 | D1 验收；无 `[object Object]` |
| **S3** | Memory 家 | 家目录 BML/ACTMEM/MEMRULES + `/api/memory`。生产 `open` / GUI / 工具只打新家 | S1 | D2 验收；不碰 ledger |
| **S4** | Skill 家 | `{config_dir}/skills/` + `skill_read` + SkillProposal。`SkillsLoader` / zip 只打新家 | S1 | D3 验收；distill 不静默写盘 |
| **S5** | 卸旧面 | 删已无调用者的旧路由/符号；改锁旧行为的测试 | S2–S4 绿 | §3 扫描族零命中 |
| **S6** | 证明与桌面 | `just cognitive-clean-break-check`；真机 | S5 | §6 / §7 |

S2/S3/S4 **代码可并行写**；**合入顺序** AutoDream 必须 S3 再 S4（先 ACTMEM 整理，再 SkillProposal）。  
各片合入时：该域生产调用点只打新家。旧路由可暂留死代码到 S5，**禁止**再被 GUI/Agent/AutoDream 调用。  
S5 不得早于三家都可工作。

---

## 2. API / 事件 / 错误总表

新前缀是权威。旧前缀在 S5 删除。

| 域 | 前缀 | 权威稿 |
| --- | --- | --- |
| Persona / WORLD | `/api/persona` | D1 §10 |
| Memory / ACTMEM / MEMRULES | `/api/memory` | D2 §10 |
| Skill / 进化提案 | `/api/skills`、`/api/evolution` | D3 §8 |
| 危险工具 | `/api/approvals` | 保留；**无** `domain=memory` |
| AutoDream 运行 | `/api/autodream/runs` | **只跑梦**，不是 Inbox |

**WORLD 读工具：** CORE `world_read`。返回当前头 Markdown，有界 ≤1200 字。禁止 `WorldStore::project()`。

**P5 写请求工具：** DEFER `persona_request`。只建 `PersonaChangeRequest`（`POST /api/persona/requests`）。Agent 改 IDENTITY / RELATIONSHIP / REDLINE / WORLD / USER 偏好只能走它。禁止工具直写头。S5 删除 `laputa_propose_section_write`。

**事件：** 聊天 `AgentEvent` + approvals SSE 保留。Persona/Memory/Skill 变更不进旧 `LaputaEvent` 总线。需要刷新时用各自 GET。

信封 JSON 只包元数据。权威正文是 Markdown / Skill 文件。错误码以各 D 包为准，不在此重造一套。

Tauri 命令与 HTTP 同名镜像。GUI 不直读磁盘。

---

## 3. 删除矩阵

生产路径（crate 源、GUI、测试夹具、just 配方）必须归零。`docs/research` / `docs/dev/archive` / 本包 **不算** 残留。

### 3.1 必删（S5 扫）

| 族 | 代表符号 / 路径 |
| --- | --- |
| Persona JSON | `sections/*.json`、`LaputaSection.content: Value`、`formatJson`、JSON 编辑器 |
| 种子 | `initialize_sections` null、`# WORLD\n` 预写 |
| 旧保存 | `create_user_edit_proposal`、`/api/laputa/section/*/write`、`laputa_propose_section_write` |
| Memory 治理 | `MemoryGovernanceCoordinator`、`domain=memory`、`MemoryApply`、`governance.sqlite3` |
| `memory_md` | `LaputaSectionName::MemoryMd`、`MemoryPatch→MemoryMd`、左栏 `long_term` |
| 混域提案 | `ProposalType::{IdentityPatch,RelationshipUpdate,CommitmentSet,LearningNote,SopCreate,MemoryPatch,Deprecation}` 生产路径 |
| 旧提案 HTTP | `/api/laputa/proposals/**`、`/persona-workspace`、`/snapshot`、`/cognitive/:kind`、`/changelog/**`、`/events/**` |
| 旧 BML HTTP | `/api/bml/memories*` 生产调用 |
| 旧 FC | Frozen Core `serde_json::to_string` 投影 |
| 旧 Memory 适配 | `LaputaMemoryProvider` 五段 JSON、`MemoryManager` 默认、`DegradedMemoryProvider`/`CutoverMemoryProvider`/`Legacy` 当生产 fallback、`recover_memory_approvals` |
| Settings zip | `upload_skill_zip` 写 `workspace/skills` |
| changelog 回滚 | `rollback_changelog` / `laputa_rollback_changelog` |
| GUI 错名 | `working_memory` 当 STM、`openEvolutionProposal`、Memory `open-approval`、`GovernanceActionBar` |
| 旧引导 | `FIRST_RUN_ONBOARDING_BLOCK`、`WELCOME_STORAGE_KEY` 当人格完成 |
| 旧 WORLD 治理 | `WorldGovernance` 队列、`world-proposals.json`、`world-ledger.jsonl`、`WorldStore::project` 生产装配 |
| MEMRULES 错位 | `.laputa/cognitive/MEMRULES.MD` 权威、Persona 左栏 `memrules` |
| 工作区 LTM | 生产打开 `<workspace>/.laputa/memory.sqlite3` |
| Skill 旧根 | 生产打开 `workspace/skills`；`memory_distill` `fs::write` |
| 退休探测 | `persona-retire`、根 `SOUL.md`/`IDENTITY.md`/`USER.md` 探测 |
| 右栏治理 | `PersonaLifecyclePanel`、Evolution 混 Memory/Persona Inbox |
| Prompt 旧文案 | 「memory update/remove 要审批」 |

### 3.2 必留

- `/api/approvals` command/plan、M3
- 打包 `skills/` 内置
- C1–C5 骨架（内容按 D1–D3 换）
- BML 表结构（换路径，不换权威模型）
- `just laputa-clean-break-check`（mentle 门继续）

### 3.3 不是权威、S5 可不删文件

- AutoDream `reports/`、run 日志：可留磁盘，**不进** Prompt / Evolution 待审 / BML
- Notebook daily/weekly/monthly：**删除产品入口**（不再当 section 权威）；磁盘旧文件不导入
- `changelog/` `audit/` `rollback/`：人格历史改走 D1 `persona/history`；旧治理 changelog **不**当人格史

### 3.4 旧用户数据

**无迁移。** 新二进制不读、不转、不提示「已导入」：

- `.laputa/sections/*.json`
- 工作区 `.laputa/memory.sqlite3`（生产改读家目录；旧文件留在磁盘当考古）
- `workspace/skills/*`（用户若要，**自己在盘外**拷到 `{config_dir}/skills/`）

**作废** D3 文里「D4 一次性复制」那一臂。切片脚本 / 产品启动 **禁止** `fs::copy` 工作区 sqlite 或 `workspace/skills` 进新家。
- `.laputa/cognitive/*`

发布说明写清：旧文件还在磁盘上，产品当它们不存在。

---

## 4. 保护分支

只在你说 **切** 时建。建议名：

`protect/cognitive-pre-clean-break-<YYYYMMDD>`

- 从当时 `agent-diva-pro` 已验证 tip 建分支。不追旧乱 SHA。
- 只读考古，不是 runtime fallback，不长期双轨。
- 验证：该 tip 能 `just check`（或记录已知失败）；记下 SHA 进本包修订。

本包批准 **不等于** 已切。

---

## 5. 发布说明（给操作者）

必须写进发版稿：

1. 人格/记忆/能力改到整机 `{config_dir}` 一份家。换仓库还是同一个伴侣。
2. 没有导入向导。旧 `.laputa` JSON / 工作区 sqlite / `workspace/skills` 不会自动出现。
3. 第一次打开会走五文件引导（若家目录五份都不在）。
4. 记忆删除不再弹出审批。
5. 蒸馏出的 Skill 要在 Evolution 点接受才生效。
6. Approval Center 只剩危险工具。

回滚：检出保护分支二进制。不提供「新码 + 旧 JSON 混跑」。

---

## 6. 自动测试与扫描

每片：`just fmt-check`、相关 crate `cargo test`、GUI 若动了加 vitest。  
S5 后：`just check`、`just test`（或记录预存失败）、新门：

`just cognitive-clean-break-check`

扩 R4 形状：生产源扫 §3.1 族；allowlist 研究/归档；自测插入 `MemoryMd` 必须红。  
现 `laputa-clean-break-check` 保留。

B 层：同片改掉锁旧合同的测试（R4 §3 表），不得留着挡 clean break。  
`[object Object]` 证伪测试留到 Markdown 链路测试替代后再删。

---

## 7. 真机验收

1. 新 profile：五文件全缺 → 引导 → 一次完成 → 第一次聊天 Frozen Core 是 Markdown。
2. 中央编辑器是 MD，不可能 `[object Object]`。
3. Memory 增删改查、ACTMEM 入口、MEMRULES 设置，都不开 Approval。
4. 发言后 `actmem` 能读到 Pulse；本轮助手结束后立刻有 Recap；另一 session 也能读同一份。
5. Evolution 只有 Skill + 待审；接受后 `{config_dir}/skills/<slug>/SKILL.md` 出现；distill 无人接受不落盘。
6. 危险工具 Approval 仍走得通。
7. 旧工作区副本：不出现自动导入条；Persona 不读 JSON section。
8. 子代理不读不写人格 / ACTMEM / BML / Skill。

---

## 8. 恢复演练

切保护分支后：用该分支二进制打开**拷贝**的旧工作区，确认旧 GUI 仍能开（考古）。  
新 tip 打开同一拷贝：不写回旧 JSON、不混家。  
演练记录进 `docs/logs/`，不进产品。

---

## 9. D4 验收

1. 切片顺序能独立回滚，S2/S3/S4 不互相抢旧读。
2. 删除矩阵可机器扫；误报面写清。
3. 保护分支有名字和触发语（切），没有提前建。
4. 发布说明包含「无迁移」。
5. 真机八条可勾。
6. 没重开 D1–D3 产品。

你批准本包之后，**先说切**，再领 S1 生产范围。
