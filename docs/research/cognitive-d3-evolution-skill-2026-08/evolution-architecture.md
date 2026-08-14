# D3 — Evolution、SOP 与 Skill

- 状态：`Approved / Implementation Pending`
- 日期：2026-08-15
- 性质：D3 架构合同。按 **D6** 把能力进化收成可管理的 Skill 文件 + 专用人审。
- **不是** Architecture Gate，**不授权**改生产代码。
- 用户批准：`2026-08-15`（对话「正确，批准，继续」）。

不得重开：D6/D7、P5/P19、S8/S9、已批准 D1/D2、P22 整机一份家。  
**不再以 GA 为进化参考。** 密度尺子可对照；进化叙事走 AutoDream → 提案 → Skill 文件。

---

## 0. 做什么 / 不做什么

**做：** SOP 与 Skill 的关系；整机 Skill 目录；SkillsLoader 新根；SkillProposal 人审；触发（AutoDream / `memory_distill`）；证据字段；启用/禁用；C1 密度约束；Evolution GUI；`/api/evolution` 契约。

**不做：**

| 留给 | 不在 D3 定 |
| --- | --- |
| D1 | 人格审查、WORLD |
| D2 | ACTMEM 整理算法、BML CRUD |
| D4 | 删除切片、保护分支、WORLD 读工具名 |
| 仍不做 | Candidate→Published 两阶段晋升机；GA 定时假离开；子代理写 Skill；把报告当 Skill |

禁止：`SopCreate→Identity`、Memory/人格进 Evolution Inbox、静默 `fs::write` Skill、Skill 当第二套 LTM / 第二套人格。

---

## 1. SOP 与 Skill：一个对象

R1 假设 A（证据最支持）。D6「提炼结果即 SOP/Skill 文件」落成：

- 用户只看见 **Skill**。
- 权威文件是 `SKILL.md`。SOP 是**写法**：极简步骤、前置、坑、可复用边界。不是第二份权威，不另开 `*.sop.md` 库。
- 包里可以有可选脚本/资源，**不是**第二种能力对象。没有脚本也可以是合法 Skill。
- **不**做「先 SOP 草稿、再晋升 Skill」状态机。人审中的提案就是草稿；接受后就是 Skill。

否决假设 C（两套对象两套生命周期）。假设 B 若以后要做，另立项；v1 默认路径无草稿机也能工作。

---

## 2. 目录与加载

```text
{config_dir}/skills/<slug>/SKILL.md     ← 伴侣能力（本包写死）
{config_dir}/skills/<slug>/…            ← 可选附属文件
{config_dir}/skills/requests/<id>.json  ← SkillProposal
仓库 skills/<slug>/SKILL.md             ← 内置，只读于产品（发版带来）
```

`slug`：`[a-z0-9][a-z0-9-]{0,62}`。**保留名禁止**：`requests`、`history`、Windows 设备名。同名时 **家目录覆盖内置**。

家目录存在且 `enabled: false`：**不**回落到内置（用户是在停用这份能力）。  
硬删家目录同名包之后：内置同名重新可见。  
缺省无 `enabled` 字段（旧文件/内置）= **true**，避免切日卸掉现网 Skill。

索引与冲突键按 **目录 slug**，不用 frontmatter `name`。

今日 `{config_dir}/workspace/skills/` 与 `memory_distill` 直写是旧落点，runtime **不读**。D4：**不复制**，用户盘外自拷或重写。Settings zip：改写入家目录并走同一写核，或删除上传。

`SkillsLoader` companion 根改为 `{config_dir}/skills/`。内置根仍是打包 `skills/`。  
Skill 文件变更：本机 **全部 session** 的 Skills 缓存失效，不按 `workspace_id`。

不预建空 `SKILL.md`。

---

## 3. `SKILL.md` 形态

YAML frontmatter + Markdown 正文。

```yaml
---
name: retry-http
description: 对幂等 HTTP 失败做有界重试
enabled: true
always: false
---
```

正文建议（非强制模板引擎）：前置、步骤、坑、不要做什么。  
`always` **只认 YAML 顶栏**。接受/直存时剥掉 `metadata.nanobot.always` / `openclaw.always`，禁止第二条后门。  
AutoDream / distill 的 `proposed_markdown` 接受时 **强制** `always: false`。  
用户可把已有 Skill 改成 `always: true`，但稳定前缀 always 全文合计软顶 2000 字；超出部分改走索引。单文件正文 >4000 即使 `always` 也只进索引。  
`enabled: false` 优先于 `always`：不进 C1。

用户可在 Evolution 页直编**已存在** Skill（D0 §3 已补执照）。CAS 用内容哈希。同文 no-op 不涨历史。PUT **禁止创建**新 slug（新建走提案）。改 `enabled` / YAML `always` 也是直存。

字数：单文件软顶 4000。

---

## 4. SkillProposal（唯一进化业务对象）

独立于 PersonaChangeRequest、独立于 Approval、独立于旧 `EvolutionProposal`。

物理：`{config_dir}/skills/requests/<id>.json`。  
状态：`pending | accepted | rejected | stale`。

**一 slug 一条 pending。** 再提交 → `skill_request_exists`。多条能力必须多个 slug，或先拒旧再提。

字段：

| 字段 | 要求 |
| --- | --- |
| `id`, `slug`, `title` | 必填 |
| `proposed_markdown` | 完整拟写入的 `SKILL.md` |
| `evidence` | **创建即须非空**。每项至少一项：`session_key` **或** `actmem_pointer` **或** `autodream_run_id`；可选 tool 名、artifact |
| `attestation` | 仅 `user_request`：人写的「我确认」。**不是** evidence |
| `base_hash` | 覆盖已有 Skill 时为当前头哈希；新建为 `0` |
| `source` | `autodream` \| `distill` \| `user_request` |
| `reason` | 短理由 |

空 `evidence`：**创建就拒** `skill_evidence_required`，不是等到接受。  
v1 **不**回放工具、不核「真的成功了」。不要在对外文案里叫 Action-Verified。密钥形态在 **accept 与用户 PUT** 都扫，命中 → `skill_secret_rejected`。失败任务能否进权威靠人拒，代码不拦。

接受（一次原子）：

1. `pending`，且 `evidence` 非空。
2. 目标文件哈希仍等于 `base_hash`（或仍不存在），否则标 `stale` 并拒接受（`skill_request_stale`）。
3. 对 autodream/distill 强制 `always: false` 并剥 runtime always。
4. 写入 `SKILL.md`（`atomic_write`），再标 `accepted`，再失效本机 Skills 缓存。头写成请求没标 accepted = 孤儿头，启动时按文件存在视为已落地，请求标 accepted。
5. PUT 冲突用 `skill_hash_conflict`（409），不要和 stale 混用。

拒绝：只终结请求。不写 Skill。  
用户直编 Skill 成功后，该 slug 上 pending → `stale`。

AutoDream / `memory_distill` **不可 apply**。

---

## 5. 谁写、谁触发

| 谁 | 做什么 |
| --- | --- |
| AutoDream | 整理 ACTMEM（D2 直写）。然后**可以**诞生 0..N 条 SkillProposal。不可 apply、不可静默写盘 |
| `memory_distill` | **只**建 SkillProposal（D7）。取消新建静默 `fs::write` |
| 用户 Evolution 页 | 直存已有 Skill；接受/拒绝提案；手写新建走 `user_request`（要 evidence **或** `attestation`，二者至少一项） |
| 聊天 Agent | 不得 `file_write` 家目录 skills；只能调 distill |
| 子代理 / cron | 禁写 Skill、禁建提案 |
| 内置技能 | 只随发版更新，不走人审 |

触发**没有**定时进化环、没有任务结束强制结晶。批处理靠 AutoDream；会话内靠用户或模型决定调 distill。

写提案前（AutoDream 产 Skill 段、distill 执行前）注入 MEMRULES 全文（S9）。

---

## 6. 上下文密度

C1 `AgentRulesAndSkills` 槽保留，但 D3 锁内容纪律：

- 稳定前缀：**所有**已启用 Skill 的短索引（slug + 一行 description）。
- 另加：`always: true` 且正文 ≤4000 的全文；合计超过 2000 字的 always 全文截到预算，其余只留索引。
- 模型要看非常驻全文：CORE 只读工具 **`skill_read(slug)`**，只开 `{config_dir}/skills/<slug>/SKILL.md` 与打包根。禁止 `file_write` 这两处。不要靠 workspace `file_read`（家目录在 sandbox 外）。
- 巩固的目的：下次少探索。索引必须能发现新 Skill。

禁止把全部 `SKILL.md` 正文常驻。

---

## 7. GUI

Evolution 工作区只做能力：

1. **Skill 列表**：搜索、启用/停用、打开编辑、历史、删除（软：`enabled: false`；硬删要确认，不进回收站 v1）。
2. **待审**：只读拟稿 + evidence 列表。接受 / 拒绝。stale 禁接受。
3. **当前 Skill**：Markdown 编辑（可复用 D1 CM6 最小集，不是 Persona 三态搬家）。

删除：通用 Proposal Inbox、MemoryPatch 卡、人格提案、把 AutoDream run 当主对象。Run 日志若留，只当调试，不是权威。

Persona / Memory 页不列 Skill 提案。

---

## 8. Manager / Tauri

| 方法 | 路径 | 作用 |
| --- | --- | --- |
| GET | `/api/skills` | 家目录 + 内置（标 source） |
| GET/PUT | `/api/skills/:slug` | 读 / 用户直存已有（CAS）；PUT 创建 409 |
| GET | `/api/skills/:slug/history` | 快照列表 |
| GET | `/api/skills/:slug/history/:n` | 一份快照 |
| DELETE | `/api/skills/:slug` | 硬删家目录包（确认）；内置只禁停用不删 |
| POST | `/api/skills/:slug/disable` | `enabled: false` |
| GET | `/api/evolution/requests` | 提案列表 |
| GET | `/api/evolution/requests/:id` | 单条 |
| POST | `/api/evolution/requests` | 建 pending |
| POST | `/api/evolution/requests/:id/accept` | 原子接受 |
| POST | `/api/evolution/requests/:id/reject` | 只终结 |

错误码：`skill_request_exists`、`skill_request_stale`、`skill_evidence_required`、`skill_secret_rejected`、`skill_slug_invalid`、`skill_hash_conflict`。

旧 `/api/laputa/proposals` 对 Skill/Memory/Persona 的混用 → D4 删除面。

---

## 9. 历史

每次成功接受或用户直存：`{config_dir}/skills/<slug>/history/<n>.md` 完整快照。  
载入历史 = 草稿，再保存才新头。不裁剪。不进 Prompt。

---

## 10. AutoDream 报告

报告 / Notebook **不是** Skill、不是 LTM、不是人格、不进 Prompt、不进 Evolution 待审。  
文件去留 D4。D3 只锁：不得把报告当成能力权威。

---

## 11. 给 D4 的删除面（不排期）

- `ProposalType::SopCreate` 及 `target=Identity`
- AutoDream 默认 `MemoryPatch`
- `memory_distill` 静默 `fs::write`
- EvolutionView 通用 Inbox 承载 Memory/Persona
- `workspace/skills` 当生产伴侣权威
- Settings 另写一套 Skill 副本
- 把 Skill 提案送进 `governance.db`

---

## 12. D3 验收

1. 用户只管理一种对象：Skill；接受后磁盘上是 `SKILL.md`。
2. AutoDream / distill 不能在无人接受时出现新权威文件。
3. 空 evidence 不能接受；人格/BML 提案不能出现在 Evolution 待审。
4. 新 Skill 默认不 `always`；前缀只有短索引。
5. 家目录 Skill 跨 git 仓库仍在（P22 同一份家）。
6. 没规定 Persona 目录、ACTMEM schema、删除切片。

通过本包 ≠ 改生产。还要 D4 + 你叫切。
