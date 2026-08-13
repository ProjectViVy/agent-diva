# D2 — Memory、BML、ACTMEM、SessionCheckpoint 与上下文

- 状态：`Approved / Implementation Pending`
- 日期：2026-08-15
- 性质：D2 架构合同。把 S1–S9、D0、已批准 D1 收成可实施的存储、触发、工具与 GUI。
- **不是** Architecture Gate，**不授权**改生产代码。
- 用户批准：`2026-08-15`（对话「同意」）。

不得重开：S1–S9、P20/P21/P22、D0-A/B/C、D1 人格目录与三态 API、C1–C5 现合同。  
R2 当时 Hold 的物理权威：产品已选 **独立 Markdown `ACTMEM.MD`**（S8），本包不再重开 A/B/C/D 库选项。

---

## 0. 做什么 / 不做什么

**做：** BML 整机落点与直写 CRUD；人格 kind 失去权威；L1 过滤；ACTMEM 正文/胶囊/CAS/触发；CORE `actmem` + DEFER 管理名单；SessionCheckpoint 改名与 GC；MEMRULES 搬家与写入时刻注入；Memory GUI；`/api/memory` 契约。

**不做：**

| 留给 | 不在 D2 定 |
| --- | --- |
| D1 已定 | `{config_dir}/persona/`、PersonaChangeRequest、WORLD P5 |
| D3 | SOP/Skill 文件形态、Evolution GUI、`memory_distill` 提案箱长相 |
| D4 | 删除切片顺序、保护分支 SHA |
| 仍不做 | STM→BML 自动晋升；子代理进 ACTMEM；Pulse/Work **自动装配**进 Prompt |

禁止：`memory_md` 双读、Memory 走审批、ACTMEM 进 BML/L1、把 `WorkingMemory` 当 STM、cron 偷写交互活动集、预建空 `ACTMEM.MD` / 空 `memory.sqlite3` 当「已初始化」。

---

## 1. 整机家（与 D1 对齐）

```text
{config_dir}/                         ← P22 家（~/.agent-diva）
  persona/                            ← D1
  actmem/
    ACTMEM.MD                         ← 本包权威头
    capsules/<safe_session>_<unix>.md
  memory/
    memory.sqlite3                    ← BML（本包写死文件名）
    MEMRULES.MD                       ← S9；缺则内置 R1–R7，不预建空文件
```

工作区 `.laputa/memory.sqlite3` 与 `.laputa/cognitive/MEMRULES.MD` 是旧落点，**不读**。  
目录 `actmem/`、`memory/` 可建空。权威文件在**第一次真实写入**时创建。

`governance.db` 仍留工作区，不进这家。

---

## 2. BML（LTM）

权威：`{config_dir}/memory/memory.sqlite3` typed SQLite + FTS5。schema 沿用现表，不新开 crate（抽层仍按既有 BML 调研，不挡本包）。

### 2.1 写

CRUD **直写**。`memory_add` / `memory_update` / `memory_remove` 都立即生效。  
删除 = 软删 + 墓碑。更新走记录 CAS（现 `record_revision`）。  
禁止再创建 `EvolutionProposal`、禁止 `MemoryGovernanceCoordinator`、禁止 Approval `domain=memory`。

模型发起的 add/update/remove：**调用执行前**把 MEMRULES 全文放进当轮提示（对标 GA L0 / S9）。  
禁止只靠「写完贴在 tool_result / 下一轮」。用户 GUI 直写不灌手册。

### 2.2 人格 kind

`MemoryRecordKind::{Identity,Relationship,Commitment,Preference}`：

- **不得**再写入。新 `put` 这些 kind → 拒绝。
- **不得**进 L1、Memory 默认列表、AutoDream 人格输入、Frozen Core。
- 库里旧行当墓碑留下，D4 可扫；本包不迁移进七文件（P8 无导入）。

允许的生产 kind：`LongTerm`（及日后 D2 登记的非人格 kind）。`WorkingMemory` 见 §6，改名后仍不是 LTM。

### 2.3 L1

C1 的 `MemoryPolicyAndIndex` 槽保留。内容只能是：

- 极短政策指针（可改成「写记忆先读 MEMRULES」，不再声称「update/remove 要审批」）。
- Applied、`session_id IS NULL`、非人格、非 SessionCheckpoint 的 **存在性索引**（有界）。

禁止：全文记录、MemoryMd、人格 kind、ACTMEM、WORLD。  
PrefetchRecall 仍是 TurnVolatile，只召回 BML LongTerm。

### 2.4 打开失败

BML 打不开：Memory CRUD 失败可见；ACTMEM 文件读写必须仍可用（不同文件）。不得把人格/ACTMEM 降级进 sqlite。

---

## 3. ACTMEM 文件

R2 Hold 1 选 **A. 独立 Markdown**（S8 已冻）。否决：BML 新 kind、独立 sqlite、升格 WorkingMemory、Plan store。

### 3.1 头文件

`{config_dir}/actmem/ACTMEM.MD`：

```markdown
---
revision: 1
updated: 2026-08-15T00:00:00Z
---

## Pulse

- [2026-08-15T00:00:00Z] gui:abc: 短用户原话

## Work

### Goal
### Open
### Next
### Constraints
### Pointers
```

- **Pulse**：环形近讯。只收**短用户原话**（原文可进，助手全文/整段对话不进）。单条上限 280 字（超出截到 280，机械）。整节超 1600 从最老条删。计数与 P14 相同（trim 后一字一计）。
- **Work**：活动集。Goal / Open / Next / Constraints / Pointers。v1 不单开「来源 / 自动化状态 / Skill 引用」节；Skill 引用可写在 Pointers。整节超 1600 → 写核 **拒绝** `actmem_cap_exceeded`，由整理者删完成/失效项后再写。**禁止**装配层静默截断。
- Pointers 只许最小指针（BML id、WORLD claim、session key、artifact、外链），不许贴 BML 正文。
- v1 **无** ACTMEM 版本历史 / 撤销。误操作靠用户再改回去或从最近胶囊手修（显式砍 S5 的「历史/撤销」选项，留下可再编辑）。

未登记的三级标题当普通 Work 文本，不另开权威节。

### 3.2 胶囊

`{config_dir}/actmem/capsules/<safe_session>_<unix>.md`。单文件 ≤800 字。  
`safe_session` 把 `:` 换成 `_`。可带原 session_key，避免撞名。

胶囊是归档，不是第二份头。不自动升回 Pulse/Work，不进 BML。

### 3.3 CAS（一个文件、两个写频率）

头文件 front matter `revision` 单调 +1。所有写者走**同一写核**：

1. 读当前；文件不存在 = `rev=0` 空 Pulse/Work。`GET` / `actmem` 读缺失头返回这份空视图，**不**创建文件。
2. 请求带 `base_revision`。
3. `base != 当前` 时：
   - **Pulse 追加**（系统发言）：重读最新头，只把新条接到最新 Pulse，Work 原样保留，**自动重试一次**。仍失败则打日志、丢掉这一条近讯，不覆盖 Work。
   - **Work 整理**（Agent / AutoDream / DEFER）：若盘上相对 base **只变了 Pulse**，重读后把本次 Work 写到最新头上（保留新 Pulse），**自动重试一次**。Work 本身冲突 → 拒绝，调用方重读。
   - **用户 PUT**：不自动重试，409 `actmem_revision_conflict`。
4. 规范化后写 staging，再 `atomic_write`。失败不涨号。
5. no-op 不涨号。

跨进程仍是 TOCTOU：不宣称「CAS 等于串行化」。重试一次是为了保住「发言即写」和「必须能整理」同时成立，不是跨进程锁。

第一次真实 Pulse 或用户/整理写入才创建 `ACTMEM.MD`。

---

## 4. 谁写 ACTMEM、何时写

| 写者 | Pulse | Work | 胶囊 | 过手册？ |
| --- | --- | --- | --- | --- |
| 系统：用户发言 | **是**（该条短原话） | 否 | 否 | 否 |
| 系统：该会话空闲 10 分钟 | 否 | 否 | **是**（≤800） | 否 |
| 聊天 Agent 日常维护 | 否 | 经 DEFER 工具 | 否 | **是**（改 Work 算写记忆） |
| AutoDream | 否 | **必须整理直写** | 可折叠 | **是** |
| 用户 Memory 页 | 可改（GUI，非工具） | 可改 | 可看/可删 | 否 |
| 子代理 | 禁 | 禁 | 禁 | — |
| cron / heartbeat | 禁 | 禁 | 禁 | — |

空闲计时：该 `session_key` 最后一条**用户**消息起 10 分钟无新用户消息。只对交互 session。session 已结束则取消该定时器；**不**补写胶囊。`api:cron:*` 不写胶囊、不写 Pulse。

CLI cron 若与交互共享 `cli:direct`：仍**禁止** cron **turn** 写 ACTMEM（R2 V8）。cron turn **可以只读** `actmem`。

系统 Pulse 失败：见 §3.3 重试一次。仍失败打日志，不回滚用户消息。  
系统胶囊：从该会话 Pulse + 当时 Work 抽 ≤800 字；超了截到 800。AutoDream「折叠」= 把已收敛的 Work 写成胶囊并缩短头里 Work，不是第二套历史。v1 胶囊不自动 GC。

`safe_session`：与 session 文件相同的安全化规则，并在胶囊 front matter 保留原始 `session_key`。

v1 **无** STM→BML 自动晋升。`memory_add` 不得从 ACTMEM 整理器隐式调用。

---

## 5. 工具

走现成 C4/C5e，不另做挂载。

**CORE（日常常驻，仅一个）：**

- `actmem`：只读。可问 Pulse / Work / 某胶囊 / 胶囊目录。返回有界预览（建议单次 ≤1200 字），大结果进 artifact。

**DEFER（`tool_search` 才挂上）：**

- `actmem_edit_work`：改 Goal/Open/Next/Constraints/Pointers 之一；带 `base_revision`。
- `actmem_complete`：把 Open 项标完成并移出（收敛）。
- `actmem_drop`：删一条 Pulse 或 Work 项。
- `actmem_list_capsules`：目录。
- `actmem_read_capsule`：读一颗胶囊。

禁止：常驻一串 BML 式 CRUD；禁止 `actmem_write_pulse`（**工具**不得写 Pulse；系统与用户 GUI 可以）。  
`actmem_list_capsules` / `actmem_read_capsule` 只留 DEFER；CORE `actmem` 已能按参数读目录/胶囊时，不要再双挂两套更宽接口。  
BML 工具：`memory_add` / `memory_update` / `memory_remove` / `memory_search` / `memory_list` / `memory_get` 保持直写语义，**移出「要审批」文案**。哪些进 CORE、哪些 DEFER：v1 检索类可 CORE（`memory_search`、`memory_get`），变更类 DEFER（add/update/remove），避免再撑前缀。`memory_list` DEFER。

（若现网已经把 memory_* 全放 CORE：D2 合同以本段为准，实施时往 DEFER 收，不作为「先加再删」的许可。）

---

## 6. SessionCheckpoint

产品名 **SessionCheckpoint**。现行对象：`MemoryRecordKind::WorkingMemory` + `update_working_checkpoint`。

- 仍可住在 BML sqlite，但 `session_id` 必填。kind 字符串改为 `session_checkpoint`（旧 `working_memory` 行按 session 作用域继续 GC，不进 L1）。
- 工具 `update_working_checkpoint` **改名** `session_checkpoint`（或等价），挂 **DEFER**。
- **改名** GUI 标签。禁止再叫 STM，禁止出现在 Memory 的 ACTMEM 入口。
- 进 Prompt：仅 TurnVolatile，有块才进。信任标记不得再写成 AppliedAuthority 冒充 LTM。
- `on_session_end`：物理删该 session 的 SessionCheckpoint。**不**删 ACTMEM/BML LTM/人格。
- **Reset** 该 session：也必须删其 SessionCheckpoint（修 R2 L2：reset 后旧便签复活）。
- Delete session 文件：同时删对应 SessionCheckpoint，避免孤儿。
- `run_startup_gc`：**接线**，只扫 SessionCheckpoint / `session_id IS NOT NULL` 且 session 文件已不在的行。禁止传入空列表扫光一切。禁止碰 ACTMEM。

CanonicalCheckpoint：保持 C1–C5。Compact 成败都不改 ACTMEM。

---

## 7. 上下文投影（不重开 C1–C5）

| 段 | D2 合同 |
| --- | --- |
| Frozen Core | D1，不装 ACTMEM |
| MemoryPolicyAndIndex | L1 索引 + MEMRULES **指针**（几行）。无手册全文，无 ACTMEM 正文 |
| CORE tools | 含 `actmem` 查询 + 有界 BML 读 |
| CanonicalCheckpoint | 不变 |
| TurnVolatile WorkingMemory | SessionCheckpoint，不是 STM |
| PrefetchRecall | 仅 BML LongTerm |

写记忆时刻（BML 变更、`actmem_edit_work` / complete / drop、AutoDream 整理开始、蒸馏提案生成前）：MEMRULES 全文进**执行前**提示。不进稳定前缀。不进「已经写完」的 tool_result 充当唯一入口。

---

## 8. 失败与并发

| 场景 | 合同 |
| --- | --- |
| ACTMEM 写失败 / CAS 冲突 | 盘上旧文留下；调用方可见错误 |
| 过期自动写（旧 rev） | 拒绝 |
| 用户 GUI 修正 vs Agent 整理同时 | 用户带新 rev 先成功；Agent 旧 rev 失败，须重读 |
| BML 库坏 | CRUD 失败；ACTMEM 仍读文件 |
| ACTMEM 头损坏 | 读失败可见；不自动用胶囊拼回头；用户可从最近胶囊手修 |
| 两 channel 同一家 | 同一份 ACTMEM；冲突走 §3.3 重试/409 |
| 新 session / 另一 channel | 看见同一头文件（R2 V1/V3） |

---

## 9. Memory GUI

Persona 不出现。Memory 页：

1. **BML** 列表 / 详情 / 直改 / 软删。无「提交审批」。kind 筛选不含 SessionCheckpoint 冒充 STM，不含人格 kind。
2. 右上角 **ACTMEM 入口**（S6）：进独立工作区，展示 Pulse、Work 五节、胶囊目录、预算占用、最近写入时间。用户改 Pulse/Work 直存（带 rev）。完成 Open、删条、看胶囊。无 Approval。
3. **MEMRULES 设置**：编辑手册，直存。只给人。

窄屏：BML / ACTMEM / 设置 用标签切，不塞进 Persona。

---

## 10. Manager / Tauri

新前缀 `/api/memory`。旧 `/api/bml/memories/:id/remove` 只返回 proposal 的语义 **删除**。

| 方法 | 路径 | 作用 |
| --- | --- | --- |
| GET/POST/PATCH/DELETE | `/api/memory/records[/:id]` | BML CRUD，直写 |
| GET | `/api/memory/actmem` | 头 + rev |
| PUT | `/api/memory/actmem` | `{pulse?, work?, base_revision}` 用户直存 |
| GET | `/api/memory/actmem/capsules` | 目录 |
| GET | `/api/memory/actmem/capsules/:name` | 一颗 |
| DELETE | `/api/memory/actmem/capsules/:name` | 用户删胶囊 |
| GET/PUT | `/api/memory/memrules` | 手册；PUT 只认用户会话 |

错误码：`memory_revision_conflict`、`actmem_revision_conflict`、`actmem_cap_exceeded`、`memory_kind_forbidden`、`bml_unavailable`。

系统 Pulse / 胶囊 **不走** HTTP，走 AgentLoop / idle 定时器。

---

## 11. MEMRULES

路径 `{config_dir}/memory/MEMRULES.MD`。缺文件：读用内置默认；用户第一次保存才落盘。

v1 默认条文相对今日 `DEFAULT_MEM_RULES_TEXT`：**改掉 R4「high-risk 要审批」**。Memory 直写。证据门仍是建议（R1 advisory），不挡住 `memory_add`。WORLD 入门（R6）仍约束 WORLD 写核。其余 R1–R3/R5/R7 语义保持。

v1 只给人改。WORLD 读工具名交给 D4。

BML sqlite：第一次真实 CRUD 才 `open` 创建文件。空库不是人格/ACTMEM 初始化标记。

---

## 12. 给 D4 的删除面（不排期）

- `memory_md` / `MemoryMd` / `MemoryPatch→MemoryMd` / Persona 左栏 long_term
- `LaputaMemoryProvider` 五段 JSON 权威块
- Memory remove/update → proposal / Approval
- GUI `working_memory` 当 STM
- Prompt 里「high-risk memory 要审批」
- 工作区 `.laputa/memory.sqlite3` 当生产 LTM
- `.laputa/cognitive/MEMRULES.MD` 当权威
- `memory_add` 写人格 kind

---

## 13. D2 验收

1. BML 路径在 `{config_dir}/memory/memory.sqlite3`，CRUD 不审批。
2. 新 session / 另一 channel 能通过 `actmem` 读到同一份头。
3. 发言后 Pulse 有短原话；10 分钟空闲后有胶囊；正文不进稳定前缀。
4. SessionCheckpoint 改名；reset/delete/end 清它，不清 ACTMEM。
5. 人格 kind 与 ACTMEM 都不进 L1。
6. 没规定 Skill 文件形态、Persona 目录、删除切片。

通过本包 ≠ 改生产。还要 D3/D4 + 你叫切。
