# D1 — Persona、WORLD、首次初始化与历史

- 状态：`Design Draft / Awaiting User Review`
- 日期：2026-08-14
- 性质：D1 架构合同。把 P1–P22 和 D0 收成可实施的目录、revision、引导、审查与 API。
- **不是** Architecture Gate，**不授权**改生产代码。

不得重开：P1–P22、S8/S9、D0-A/B/C、P20 三条车道。  
依据：R3 `revision-diff-options.md`、`markdown-workspace-technical-evaluation.md`、
`persona-authority-inventory.md`。本包**选出** R3 当时 Hold 的赢家。

---

## 0. 做什么 / 不做什么

**做：** 整机人格目录；种类登记表；当前头 + 永久历史；CAS 直存；PersonaChangeRequest；
五文件原子引导与 incomplete 修复；Frozen Core 改读 Markdown；WORLD 与六份共用文档引擎、
写走 P5 / 读走工具；GUI 左七 + 中央三态；Manager/Tauri 契约。

**不做：**

| 留给 | 不在 D1 定 |
| --- | --- |
| D2 | `memory.sqlite3` 打开、ACTMEM 工具、MEMRULES 搬家实现、SessionCheckpoint |
| D3 | SOP/Skill 文件形态、Evolution GUI |
| D4 | 删除切片顺序、保护分支 SHA |
| 仍不决定 | Agent **何时主动想改**人格（只冻「一旦要写审查集就必须建请求」）；DREAM 欲望量表；默认章节模板正文 |

禁止：JSON 正文、预种子、`project()` 当默认上下文、MEMRULES 进 Persona、第二套伴侣、
双读旧 section、把 changelog rollback 当人格历史。

---

## 1. 整机家与目录

`{config_dir}` = 今日 `ConfigLoader::config_dir()`，默认用户主目录下 `.agent-diva`
（Windows：`%USERPROFILE%\.agent-diva`）。这就是 P22 的那一套家。

```text
{config_dir}/
  persona/                      ← 七文件同一目录（本包写死）
    IDENTITY.MD
    RELATIONSHIP.MD
    REDLINE.MD
    USER.MD
    WORLD.MD
    DREAM.MD                    ← 缺席合法，禁止预建
    DARK.MD                     ← 缺席合法，禁止预建
    history/<KIND>/<n>.md       ← 完整快照
    history/<KIND>/<n>.diff     ← 相对上一版 unified
    history/<KIND>/log.jsonl    ← revision 元数据
    requests/<id>.json          ← PersonaChangeRequest
  actmem/ACTMEM.MD              ← S8，D1 不创建
  memory/MEMRULES.MD            ← S9，D1 不创建
  memory/                       ← BML 与人格同父；`memory.sqlite3` 文件名由 D2 写死
```

D1 可以创建空目录 `persona/` 和 `persona/history/`、`persona/requests/`。
**不得**预创建任何权威 `.MD`，不得预建 BML / ACTMEM / MEMRULES。

工作区 `.laputa/sections/*.json` 与 `.laputa/cognitive/{WORLD,MEMRULES}.MD` 是旧落点，
产品删除。新 runtime **不读**它们。

---

## 2. 种类登记表

产品持有、代码内静态表。用户 / Agent / 配置不能改。加第八种 = 先改 P18 再加一行。

| kind | 文件 | 引导 | Frozen Core | 用户直存 | Agent | AutoDream |
| --- | --- | --- | --- | --- | --- | --- |
| identity | `IDENTITY.MD` | 是 | 200 | 是 | P5 | P19 提案 |
| relationship | `RELATIONSHIP.MD` | 是 | 120 | 是 | P5 | P19 提案 |
| redline | `REDLINE.MD` | 是 | 200 | 是 | P5 | **禁** |
| user | `USER.MD` | 是（只偏好） | 160 | 偏好直存 | 观察 P16 直写；偏好 P5 | 只观察块 P19 |
| world | `WORLD.MD` | 是 | **不进** | 是 | **读工具 / 写 P5** | 只新 claim P19 |
| dream | `DREAM.MD` | 否 | 10 | 可编（defer） | P16 直写 | **禁** |
| dark | `DARK.MD` | 否 | 60 | 可编（defer） | P16 直写 | P19 提案 |

正文上限见 P14。未登记文件名出现在 `persona/` 下：忽略，不当权威，不进导航，不进 FC。

`USER.MD` 观察块 vs 偏好块：固定英文 H2。

```text
## Preferences
…用户自述…

## Observations
…Agent 观察…
```

没有这两个 H2 的已有正文，**整段视为 Preferences**（这是「第三区」的唯一解释）。  
引导 / repair 写入 USER 时，服务端必须包进 `## Preferences`（观察节可省略）。  
Agent P16 直写 USER：服务端做子树比较，`## Preferences` 有改动 → `persona_kind_forbidden`。只许改 `## Observations`（没有则创建该节）。

`DARK.MD` 固定：

```text
# DARK

## FEAR
## SHADOW
```

---

## 3. 当前头、revision、历史

R3 §2 选 **A：每对象一个 `.md`**。否决 sqlite 文档库（和 BML 混权威）、否决继续 JSON。

R3 §3 选 **C：单调序号 + 内容哈希**。

- `revision`：每文件从 1 起的整数，成功写入 +1。不重号、不回收。
- `content_hash`：规范化正文（NFC + 统一 `\n` + **trim 首尾**空白，与 P14 计数同一规范化）的 SHA-256。只做校验和 no-op，**不作主键**。
- 同文再保存：hash 相同 → **no-op**，不新开 revision（P12）。
- 「载入旧版再保存」：若正文与当前头 hash 相同仍是 no-op；若用户改了一个字，新头序号继续往上，中间历史不动。

R3 §4 选 **A：每版本完整快照**。不存纯 diff 链。体积换可还原。

R3 §5 选 **A：写时算行级 Myers unified，存进 `<n>.diff`**。GUI 解析同一份，不另算一套（避免算法升级改历史观感）。第一版不做词级。

R3 §8 选 **A：当前头一个 `.md` + `history/<KIND>/`**。不复用 `changelog/<id>.json`。

`log.jsonl` 每行是元数据。`snapshot` / `diff` 是相对 `history/<KIND>/` 的文件名（`1.md`、`1.diff`），**不**内嵌正文。

`source` ∈ `user_direct | agent_p16 | agent_p5_accepted | autodream_p5_accepted | init | history_resave`。

回滚：没有 restore-in-place API。GUI「载入到编辑器」只读快照到草稿。禁止调用旧 `rollback_changelog`。

---

## 4. 直存与 CAS

用户保存 / P16 直写 / 接受审查，都走同一写核：

1. 读当前头。不存在且 kind 允许缺席（dream/dark）→ 当空头 `rev=0`。引导五种不得经此「创建空文件」。
2. 规范化 + P14 计数。超限 **拒绝**，不截断。
3. hash == 当前 → no-op 成功。
4. 请求必须带 `base_revision`。`base != 当前 rev` → **409** `persona_revision_conflict`，草稿留在客户端。
5. 过门：写新快照与 diff → append log → **最后** `atomic_write` 当前头。头写失败则这次不算成功（历史多一条孤儿快照可在启动时核对 log 最后一条是否等于头 hash，不等则标 incomplete-on-that-file）。
6. 成功后：该 kind 上所有 `pending` 请求变 `stale`。

跨进程：v1 **纯 CAS，不加文件锁**（R3 §11 A）。两窗同编，后者 409。

WORLD 用户直存走同一写核。Agent 不得走直存（见 §8）。

---

## 5. PersonaChangeRequest

独立对象。**禁止** `EvolutionProposal` / `MemoryGovernanceCoordinator` / Approval。

物理：`{config_dir}/persona/requests/<id>.json`。

状态：`pending | accepted | rejected | stale`。

**一文件一条 pending。** 已有 pending 时再提交 → 拒绝 `persona_request_exists`。要换建议：先拒旧的。不排队。WORLD 上多条新 claim 必须打进**同一** `proposed_markdown`，不能拆成两条 pending。这是 D1 对「当前不决定」里队列问题的选择。

字段：`id, kind, base_revision, base_hash, proposed_markdown, actor, reason, created_at, state, decided_at?`。

接受（一次原子）：

1. `state==pending` 否则拒。
2. 当前 `rev==base_revision` 且头 hash==`base_hash`，否则标 `stale` 并拒接受。
3. 走 §4 写核（source=`agent_p5_accepted` 或 `autodream_p5_accepted`）。
4. 请求 → `accepted`。

拒绝：只改 `rejected`，不动头。

谁能建请求：聊天 Agent（审查集）、AutoDream（P19 允许表）。用户不走请求，用户直存。

**仍不决定：** Agent 在对话中哪一轮「想」改人格。只冻：一旦要改审查集，唯一出口是建请求，没有旁路工具。

删除旧工具 `laputa_propose_section_write`（D0 §5 已禁）。新工具名 D4 列；语义只能是「建 PersonaChangeRequest」。

---

## 6. 首次引导与 incomplete

P10 三态，只看五份用户侧文件：

| 条件 | 态 |
| --- | --- |
| 五份物理都不存在 | `uninitialized` → 引导 |
| 五份都在且每份有效 | `ready` → 永不引导 |
| 其余（缺几份、空文件、非 UTF-8、超限、半写） | `incomplete` → 修复 |

有效 = 文件存在、非空 trim、UTF-8、≤P14。  
`DREAM`/`DARK` 不参与三态。

**引导：** 只在 `uninitialized`。一次表单收五份正文。`POST initialize`：

1. 再检查仍是五份全缺，否则 409。
2. 校验五份（上限、非空）。
3. 写入 `{config_dir}/persona/.init-staging/`（只是临时目录，**不是**状态源，没有 manifest）。
4. 五份 + 五条 `rev=1` 历史在 staging 齐了，再逐个 `atomic_write` 到 `persona/`。
5. 任一步失败：不进入 ready（因为五份不会都有效）；删除 staging；保留用户五项输入。已落到 `persona/` 的半套 → `incomplete`。
6. 成功：删 staging。不建请求、不建 Approval、不建 DREAM/DARK。

ready 的充要条件永远是 P10：**五份都在且有效**。禁止 `init.manifest` 或任何第二套完成标记。

启动时若 `persona/.init-staging/` 仍在 → 删除。  
二次 `initialize` 仅当仍是五份全缺；若 `history/` 已有无头孤儿（有 `1.md` 无当前头），先把该孤儿移出或视为 incomplete 的一部分，**禁止再写第二个 `1.md`**。  
`repair` 补缺失文件：当 `base_revision=0`，写出头 `rev=1` 与首条历史。已有效文件带自己的当前 rev，本接口不得改它们。

Windows 上目录 rename 不原子，所以用 staging + 逐文件头 + 五份有效作提交点，不用整目录换上。

**修复：** 不是第二套引导。展示哪几份缺/坏，已有正文只读可见。用户补缺或改坏件后 `POST repair` 只写**缺失或无效**的那些。禁止「全部重来」一键删五份。禁止自动补默认模板。

删除：`FIRST_RUN_ONBOARDING_BLOCK`、`laputa_propose_section_write` 引导、用 `WELCOME_STORAGE_KEY` 判断人格完成。密钥向导可留，但与本三态无关。

引导必须在第一次 Frozen Core 捕获之前完成。

---

## 7. Frozen Core

会话开始读**当前头 Markdown**（不是 JSON）。按登记表投影字数（P14）。本会话缓存，中途改文件下一会话再生效。

`USER.MD` 的 FC **先取 `## Preferences`**，再按 160 字切。不要从头文件顶端切到观察。

顺序与预算沿用 C1 第二段。空投影跳过。六段（无 WORLD）都空且 `uninitialized` 不该进会话——应用先挡在引导。

WORLD 不进 FC。`WorldStore::project()` **不**接线生产 Prompt。

捕获失败：当空投影，打错误日志，不把旧 JSON section 当后备。

---

## 8. WORLD

权威就是 `persona/WORLD.MD`，和另外六份**同一文档引擎**（CAS、历史、字数、导航）。

读：工具（CORE 或现有 file 范围由 D2/D4 列名）。默认不进 Prompt。

写：

- 用户 Persona 页直存。
- Agent / AutoDream → **只许 P5 请求**。禁止工具直写头。
- 保护：`confirmed` + `source=user` 的 claim **不得**被请求覆盖。接受时在写核之前再检；违规 → `persona_world_protected_claim`。
- 唯一可解析形态与现 `WorldStore` 同构（其余字段可选、不参与保护）：

```text
## [domain] title
- status: confirmed
- source: user
```

身份按 `domain + title`。改 title / 删段 / 整文件抹掉该块 = 覆盖。AutoDream「只新 claim」= 不得改已有 `domain+title`。
- 无法解析的段落当普通 Markdown，不享受 claim 保护，但仍受 P5。
- 引导里的 WORLD 散文：整份视为用户正文。接受后的新头不得把无 `source: user` 的版本整文件换掉这份散文；v1 也可在 initialize 时把未分 claim 的原文收成一条 `## [environment] Current` + `source: user`（二选一，实现选后者须在 initialize 写核里做，不得 silently 丢用户字）。

**WorldGovernance** 队列、`world-proposals.json`、`world-ledger.jsonl`：**产品删除**。保护规则进写核，不进第二套账本。

---

## 9. GUI

左栏：只七份。更新时间、dirty、该文件 pending 数。无 `memory_md`、无 changelog section、无 MEMRULES。WORLD 可编，不是只读 pre。

中央互斥：`当前文档 | 待审变更 | 历史`。无永久右栏。

- **当前文档：** CodeMirror 6 源码 + `markdown-it` 预览（`html:false`）。预览可收起。最小扩展：`history`、`search`、`lineNumbers`、`highlightActiveLine`、Markdown 语法、line wrapping、`Mod-s`。不要语言服务、不要自绘主题体系。预览禁远程图；链接走现有 `validateLink` + 过滤后的外开。
- **待审：** 只读 before/after + 存盘的 unified Diff 高亮。接受 / 拒绝。stale 禁接受。
- **历史：** 只读列表；载入 = 草稿。无 rollback 按钮。

409 / 保存失败：保留正文、选区、滚动。切文件丢草稿要确认。

窄屏：左栏可收；中央仍是三态标签，不回到三栏审批。

---

## 10. Manager / Tauri 契约

新前缀 `/api/persona`。旧 `/api/laputa/section/:name/write` 与 persona-workspace 里的 JSON/MEMRULES **删除面**（D4 切片）。

| 方法 | 路径 | 作用 |
| --- | --- | --- |
| GET | `/api/persona/status` | `uninitialized\|ready\|incomplete` + 每文件存在/有效 |
| POST | `/api/persona/initialize` | 五正文原子初始化 |
| POST | `/api/persona/repair` | 只写无效/缺失的用户侧文件 |
| GET | `/api/persona/docs/:kind` | 头 + rev + hash |
| PUT | `/api/persona/docs/:kind` | 直存 `{content, base_revision, reason}` |
| GET | `/api/persona/docs/:kind/history` | 列表（无整包快照） |
| GET | `/api/persona/docs/:kind/history/:rev` | 快照 + diff |
| GET | `/api/persona/requests` | 按 kind 滤 |
| POST | `/api/persona/requests` | 建 pending |
| POST | `/api/persona/requests/:id/accept` | 原子接受 |
| POST | `/api/persona/requests/:id/reject` | 只终结 |

**三态 × API：**

| 态 | 允许 |
| --- | --- |
| `uninitialized` | `GET status`、`POST initialize`。其它 409 `persona_uninitialized` |
| `incomplete` | `GET status`、`POST repair`、**只读** `GET docs/:kind` 与 history。PUT / requests / accept → `persona_incomplete` |
| `ready` | 上表全部（initialize 除外，再调 409） |

`with_dir` 换根只给测试/服务，**不是**产品多伴侣。Windows 上 `identity.md` 与 `IDENTITY.MD` 是同一文件；创建和列举必须按登记表的规范大小写写，禁止再造一份小写权威。

稳定错误码（信封 JSON，**正文仍是 Markdown**）：

`persona_uninitialized`、`persona_incomplete`、`persona_revision_conflict`、`persona_request_exists`、`persona_request_stale`、`persona_cap_exceeded`、`persona_kind_forbidden`、`persona_world_protected_claim`。

Tauri 镜像同名命令。GUI 不直读磁盘。

CLI：删 `persona-retire`。需要的话以后加 `persona status|init`，本包不扩。

---

## 11. 写核调用者

```text
用户 PUT docs           → 直存
Agent P16               → 直存（仅 dream/dark/user observations）
接受请求                → 直存 + 终结请求
initialize / repair     → 批量直存（init source）
Agent/AutoDream 审查集  → 只 POST requests
WORLD 工具读            → GET docs/world
WORLD 工具写            → 禁止；必须 POST requests
```

---

## 12. 给 D4 的删除面（不排期）

- `.laputa/sections/{identity,relationship,commitment,preferences,memory_md}.json`
- `LaputaSection.content: Value`、JSON 编辑器、`formatJson`
- `create_user_edit_proposal`、Persona `MemoryGovernanceCoordinator.submit`
- `PersonaLifecyclePanel`、右栏治理、`/api/laputa/section/*/write`
- `FIRST_RUN_ONBOARDING_BLOCK`、`initialize_sections` null 种子、`# WORLD\n` 种子
- `persona-retire`、根 `SOUL.md`/`IDENTITY.md`/`USER.md` 探测
- Persona 左栏 `memrules` / `long_term` / changelog
- `WorldGovernance` 队列与 ledger 文件
- `WorldStore::project` 生产装配
- changelog rollback 当人格历史

---

## 13. D1 验收

用户评本包时看：

1. 能指出七文件的磁盘目录，且是整机一份、不是仓库 `.laputa`。
2. 保存有 CAS；冲突留草稿；no-op 不涨历史。
3. 引导三态只看五文件物理存在；无预种子。
4. 待审是 Persona 四态，一文件一条 pending，接受一步写头。
5. WORLD 与六份同引擎；读工具写 P5；无 `project()` 默认装配。
6. 没规定 BML 打开方式、Pulse schema、Skill 形态、删除顺序。

通过本包 ≠ 改生产。还要 D2–D4 + 你叫切。
