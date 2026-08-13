# STM 分层完整方案（提案）

- 状态：`Proposal / Awaiting User Review` — **不是已批准架构，不授权施工**
- 日期：2026-08-14
- 修订：同日用户同意「一份注入文件 + 胶囊不注入」；旧 Garden / `MEMORY.MD` 舍弃；文件名待选
- 性质：把跨会话不失忆拆成可执行的分层合同，供用户拍板或驳回
- 边界依据：[decision-record.md](./decision-record.md) S1–S6
- 研究依据：[R2](../cognitive-r2-stm-context-2026-08/README.md)（本提案选推荐项；R2 原文仍是选项集）
- 硬前提：**STM 不得与 BML 管理的 LTM 混权威、混存储、混 API、混列表、混 Prompt 段**

本文件若与 S1–S6 冲突，以 S1–S6 为准。本文件若与 R2 Hold 列表冲突，以「用户对本提案的批注」为准，在用户批准前仍按 Hold。

---

## 0. 一句话

STM 是 workspace 级、严格限容、实时可控的**跨会话活动层**。它记住「最近发生了什么、现在在忙什么」，不是长期事实库。长期事实只进 BML。旧对话全文留在 session transcript，需要时用工具打开，不整包注入。

---

## 1. 必须达成的效果

1. 用户在 Session A 聊了一会，在 Session B 问「刚才聊了啥」：Agent **不应失忆**，应能说出 A 里发生了什么。
2. 用户在多个 session 只发「你好」没办事：其它会话里的 Agent **应能看见这个模式**，并可以问「你发这么多你好是什么意思」。
3. STM **实时**写入、**实时**限容，不能靠「空了再压一次」才存在。
4. 以前的会话尽量「都还在」——指**目录还在、要点可回看**，不是全文永远躺在 Prompt 里。
5. 用户打开 Memory 页能看见并修正 STM；修正立即生效，不走审批。

做不到、也不做的效果：

- 把所有历史对话无损塞进每一轮上下文
- 用 STM 当第二套 BML（人物偏好、稳定事实、可检索知识库）
- 把 `memory_md` / SessionCheckpoint / CanonicalCheckpoint 改名冒充 STM

---

## 2. 硬隔离：STM 不是 BML LTM

| | STM | BML LTM |
| --- | --- | --- |
| 问题 | 现在在忙什么、刚才发生了什么 | 长期应记住什么 |
| 寿命 | 有界，随完成/过期/预算收敛 | 长期，软删/版本/来源 |
| 写入 | 自动维护 + 用户直接改 | Memory CRUD；不走 STM API |
| 审批 | 不审批 | 不审批（已冻结）；也**不**经 STM |
| 存储 | `.laputa/stm/` 独立树 | `.laputa/memory.sqlite3` |
| 类型 | 禁止 `MemoryRecordKind` | 唯一 LTM kind 权威 |
| Prompt | 近讯 + 活动集有界投影 | 只允许索引 / 按需召回 |
| 检索 | 胶囊/近讯/活动集专用工具 | `memory_search` / Recall |
| GUI | Memory 页 **STM 入口**，独立工作区 | BML 列表/详情 |
| 删除 | 清 STM 不得删 BML / transcript | 清 BML 不得删 STM |
| 晋升 | 不得把推测直接写成 LTM | 只有带证据的独立 CRUD |

禁止清单（实施时必须有测试）：

- 禁止 `memory_add` / `MemoryPatch` / `WorkingMemory` kind 写 STM
- 禁止 STM 记录出现在 BML 列表、FTS 默认检索、L1 长期索引
- 禁止 AutoDream 对 STM 走 Laputa Proposal / Governance
- 禁止 Persona 页展示 STM
- 禁止把 BML 行投影进注入文件的 Work 节当正文（只允许指针）

---

## 3. 对象：一份注入文件 + 不注入的胶囊

2026-08-14 用户同意：注入权威是**一个** Markdown；旧 Garden / `MEMORY.MD` 全舍弃。
不要做成「一份文件无限追加再原地摘要」。那样会把「五个你好」洗成「用户打过招呼」。

```text
                    ┌─────────────────────────────┐
  每轮整份注入        │  {NAME}.MD                  │  两节：Pulse + Work
  （硬顶）            │    ## Pulse  近讯           │  实时环形
                    │    ## Work   活动集          │  目标 / 回路 / 下一步
                    └─────────────┬───────────────┘
                                  │ 目录行（算进 Pulse 预算）
                    ┌─────────────▼───────────────┐
  默认不注入         │  会话胶囊 Capsules          │  每个旧 session 的压缩本
  工具打开           └─────────────┬───────────────┘
                                  │ 需要原文时
                    ┌─────────────▼───────────────┐
  已有，不是 STM     │  Session transcript         │  全文
                    │  BML LTM                    │  长期事实
                    └─────────────────────────────┘
```

| 层 | 产品名 | 权威 | 进默认上下文 | 时效 |
| --- | --- | --- | --- | --- |
| Pulse | 近讯 | `{NAME}.MD` 的 `## Pulse` | **必须进**（整文件有界注入） | 用户一发言就追加 |
| Work | 活动集 | `{NAME}.MD` 的 `## Work` | 同上 | 有目标/回路变化才改 |
| Capsule | 会话胶囊 | `.laputa/stm/capsules/{session}.md` | **不进** | 空闲或会话结束写 |
| CapsuleIndex | 胶囊目录 | 注入文件 Pulse 里的目录行，或 `capsules/INDEX.md` 投影进 Pulse | 只进目录行 | 随胶囊更新 |

`{NAME}.MD` 见 §3.1，尚未选定。S3 的目标/回路/下一步落在 Work 节。跨会话「刚才聊了啥 / 连发你好」落在 Pulse 节。旧会话「都还在」落在目录行 + 胶囊。

### 3.1 注入文件名（待用户点名）

约定：全大写 + `.MD`，与 Laputa 权威文件观感一致，但**不是** Persona 七文件，也不进种类表。
禁止：`MEMORY.MD`、`MEMORY.md`、`memory_md`、任何旧 Garden 文件名。

| 候选 | 读音/含义 | 好处 | 风险 |
| --- | --- | --- | --- |
| **`STM.MD`** | 就叫产品名 | 最短；决策/代码/口头一致；不含 MEMORY | 文件名略术语；GUI 应用中文「活动记忆」兜住 |
| **`ACTMEM.MD`** | activity memory | 好记；有「记忆」感又不是 LTM | 造词；ACT 也可读成 actor/account |
| **`STMEM.MD`** | short-term memory | 直观 | 像拼写错误；「短期」容易被理解成会话结束就清空（与跨会话持续相反） |
| **`ACTIVE.MD`** | 当前活动 | 不含 MEM，最难和 BML 搞混 | 不像一份记忆，像开关状态 |
| **`FOCUS.MD`** | 当前焦点 | 短、好记 | 偏 Work，装不下「刚才发生了什么」 |
| **`LIVE.MD`** | 活着的上下文 | 短 | 太飘，也像直播/运行状态 |
| **`BOARD.MD`** | 看板 | 活动和近讯都能往上钉 | 偏项目管理，不像记忆 |

**本提案推荐：`STM.MD`。** 若必须文件名里带 MEM、又不要 `MEMORY.MD`，退而选 `ACTMEM.MD`。不推荐 `STMEM.MD`（丑，且短期语义和 S3 打架）。

GUI / 入口文案用中文（「活动记忆」或「STM」），不要让用户只靠文件名认路。

---

## 4. 推荐物理落点

**推荐：独立目录 Markdown，不进 `memory.sqlite3`。**

理由：和 BML 物理切开，人类可 diff，Memory 页可直接打开，AutoDream 可当文件整理，不会被 BML CAS / L1 索引 / session GC 误伤。

```text
.laputa/stm/
  {NAME}.MD                # 注入权威：## Pulse + ## Work；名称见 §3.1
  capsules/
    INDEX.md
    {safe_session_key}.md
  revisions/
    {NAME}-{rev}.MD        # 注入文件有界快照，失败可回上一份
```

备选（本提案不选，仅供否决）：

| 备选 | 否决理由 |
| --- | --- |
| BML 新 `MemoryRecordKind` | 一种 store 两种寿命；必进 L1/FTS；和「不要搞混」直接冲突 |
| 升格 `WorkingMemory` | S4 禁止；session end 会删 |
| `memory_md` / `MEMORY.md` | S2 删除面 |
| Plan store | 审批域、单例、寿命不对 |
| 独立 sqlite | 可做，但第一版没有必要；人类不可读，GUI 还要再做一层 |
| 写进 CanonicalCheckpoint | compact 有损替换，且绑死单个 session |

Scope：**一个 workspace 一份** Pulse + Work。新 GUI chat、CLI、其它 channel 看到同一近讯和同一活动集。Capsule 按 `session_key` 分文件。

多 agent / 多 profile 是否再拆，本提案不扩；默认跟 workspace。

---

## 5. 每层写什么

### 5.1 Pulse（近讯）

只记录**跨会话最近事件**，不是日记，不是助手全文。

每条事件固定字段：

```text
- ts: 2026-08-14T12:01:00+08:00
  session: chat:A
  kind: user_said | session_settled | pattern
  text: 你好                         # 短用户原话；超顶则一句摘要
  ref: session:chat:A#msg-12         # 回查原文的指针
```

规则：

- **用户短句原文保留**（推荐顶：80 字）。「你好」必须原文，不能先摘要。
- 用户超长粘贴：一行摘要 + `ref`，正文不进 Pulse。
- **助手输出不进 Pulse**。助手结论进 Capsule，必要时进 Work。
- `session_settled`：该会话空闲/结束时写一行「A · 聊了发布纪要，未完成表格」。
- 环形：超条数或超字数，删**最老事件**，不对 Pulse 做原地再摘要。
- 目录：`INDEX.md` 里每个旧会话一行，算进 Pulse 预算的「目录区」，不是事件区。

推荐硬顶（可改数字，不能改「有硬顶」）：

- 事件区 ≤ 1200 字、≤ 30 条
- 目录区 ≤ 400 字、≤ 40 行
- 合计注入 ≤ 1600 字

### 5.2 Work（活动集，S3）

结构化卡片，不是流水账。

```text
# WORK

## Goals
- …

## Open
- …

## Next
- …

## Constraints
- …

## Pointers
- capsule:chat:A
- bml:rec_…
- world:claim_…
```

规则：

- 只保留**现在仍有效**的目标、回路、下一步、约束、指针。
- 完成、作废、被替代的卡片删除或移入 `revisions/`，不在当前头里堆历史。
- 禁止把 BML 正文、WORLD 全文、transcript 粘进来。指针即可。
- 无变化不升 revision（no-op）。
- 推荐硬顶：全文 ≤ 1600 字、卡片 ≤ 12 张。

### 5.3 Capsule（会话胶囊）

每个 session 一份压缩本，默认不进 Prompt。

```text
# capsule chat:A
settled_at: …
user_highlights:
  - 你好
  - 把发布说明改短
agent_outcome: 已改 changelog 口吻；表格未做
open_left: 表格
```

规则：

- **用户侧**：短句可引用原文；长内容只留要点。
- **助手侧**：只留结论、动作、未完成。禁止逐字对话录。
- 本会话还活着时，Capsule 可以是草稿；settled 后才算正式。
- 单个胶囊推荐硬顶：800 字。
- 胶囊总数超预算：最老的若干份收成 **era 一行**（例如「08-10 前 6 个会话：问候后无任务」），原文 transcript 仍在，胶囊文件可删或归档出注入面。

---

## 6. 谁写、何时写（实时控制）

```text
用户发言
  └─ 立刻 append Pulse（失败不得丢本条用户话；队列/重试）
       └─ 超顶删最老事件

本轮结束
  └─ 若目标/回路/下一步确有变化 → 更新 Work（CAS）
  └─ 若无变化 → 不写 Work

会话空闲或结束
  └─ 写/更新该 session 的 Capsule + INDEX 一行
  └─ Pulse 追加一条 session_settled

AutoDream 空闲批处理
  └─ 整理 Work（合并重复回路、关掉过期、把「连续 N 次空问好」收成一条 Open）
  └─ 折叠超龄胶囊为 era 行
  └─ 不写 BML；不对人格直写（人格只按 P19 提案）

用户在 STM 工作区改正
  └─ 直接改 Pulse / Work / Capsule，立即生效，不产生 Proposal
```

空闲压缩**不能替代** Pulse 实时追加。A 说完「你好」立刻开 B，B 的第一轮就必须已经看见那条 Pulse。

Subagent / cron：默认可读同一 workspace STM；写 Pulse 允许（它们也是事件）；写 Work 必须显式（避免把后台噪音写成当前目标）。本提案第一版：**cron/subagent 只追加 Pulse，不改 Work**，除非之后单独批准。

---

## 7. 上下文怎么进（注入 vs 工具）

### 7.1 每轮必注入（有界投影）

独立动态段，名称建议 `StmProjection`，**不要**并进：

- `MemoryPolicyAndIndex`（那是 BML 索引）
- `WorkingMemory`（session scratch，结束会删）
- CanonicalCheckpoint
- Frozen Core / Persona

投影内容 = `{NAME}.MD` 经权威侧裁切后的全文（Pulse 节 + Work 节）。合计推荐 ≤ 3200 字。超了先削 Pulse 最老事件（装配侧禁止静默丢 Work 卡片——淘汰必须发生在权威文件里，否则 GUI 和模型分叉）。

新 session 第一轮就必须读到这份投影。这是「不应失忆」的机制，不是靠模型自觉去调工具。

Cache：STM 每改一次都会打断依赖它的那一段 cache。不要放进 SessionStable prefix（现有测试要求 WM 变化不破 prefix）。放在 prefix 之后、history 之前的独立区。

### 7.2 默认不注入、工具打开

| 工具（提案名） | 作用 |
| --- | --- |
| `stm_open_capsule` | 打开某个 session 的胶囊 |
| `stm_search` | 在 Pulse / Work / 胶囊 / INDEX 里搜，**不搜 BML** |
| 已有 session 读取 | 用户要原文时读 transcript；这不是 STM 工具 |

系统提示只需要一句话：近讯和活动集已在上下文；细节用 `stm_open_capsule`，长期事实用 Memory 工具。禁止「每轮先 call stm」。

### 7.3 为什么不能「全部工具访问」

「连发你好」要求**不问也看见**。模型不知道该查别的会话，就不会查。Pulse 不注入，这个场景必挂。  
「刚才聊了啥」如果只靠工具，有时能命中、有时装失忆。必注入近讯后，这句是读 Pulse / INDEX，不是碰运气。

---

## 8. 超限怎么压

分层压，禁止对同一份文档反复摘要。

| 层 | 超限动作 |
| --- | --- |
| Pulse 事件 | 删最老一条。短用户原话优先留。 |
| Pulse 目录 | 最老会话从「一行摘要」收成 era 行；胶囊仍可工具打开直到胶囊预算也超 |
| Work | 完成/过期/被替代的卡片删掉。AutoDream 合并重复。禁止把 Work 压成散文日记 |
| Capsule | 单文件超 800 字先压助手侧。全局超标则最老胶囊变 era 行 |
| 绝不 | 把 Pulse 整段喂模型再写回 Pulse；把 transcript 折进 Work；把压出来的句子 `memory_add` 进 BML |

「以前的会话都记得」= INDEX / era 行还在，需要时打开胶囊或 transcript。不是 200 个会话全文都在投影里。

---

## 9. AutoDream 在这套里做什么

职责（与 P19/S5 一致的方向，对象形态以本提案为准）：

1. **整理 Work**：收敛回路、去重、关掉过期、补指针。直写 Work，不走提案。
2. **折叠胶囊**：超龄合并为 era，更新 INDEX。
3. **模式升级**：例如 Pulse 里连续多条「你好」且无任务 → Work 增加一条 Open「用户连续空问好，含义未明」。仍是 STM，**不是** BML。
4. **人格**：只按 P19 允许表提案，不直写，不碰红线/梦/用户偏好。
5. **禁止**：`MemoryPatch`、写 BML、把 STM 卡片晋升为 LTM、整理 Skill。

实时 Pulse **不是**等 AutoDream。AutoDream 是空闲整理器，不是近讯的唯一写入者。

---

## 10. GUI

Memory 页右上角唯一入口，例如 `[ STM · 3 个开放 · 近讯 12 条 ]`。点进去是完整工作区，不是 tooltip。

工作区三栏或三页：

1. 近讯（Pulse，可删条、可改错字）
2. 活动集（Work，可改目标/回路/下一步）
3. 会话目录（INDEX + 打开胶囊；需要原文再跳 transcript）

Persona / Evolution / Notebook / Approval **不**出现 STM。BML 列表默认过滤掉一切 STM 路径。用户改 STM 不弹审批。

---

## 11. 场景走查

### 11.1 A 聊了一会，B 问刚才聊了啥

1. A 进行中：用户句进 Pulse；若形成「改发布说明」则进 Work。
2. A 空闲：Capsule A 写下用户要点 + 助手结论 + 未完成；INDEX 多一行。
3. 打开 B：投影里已有 Pulse（含 A 的用户句和 settled 行）和 Work（若仍有效）。
4. 用户问「刚才聊了啥」：模型根据投影回答；若要细节，调 `stm_open_capsule(chat:A)`。
5. 不调用 BML。BML 里没有这次对话，也不该有。

### 11.2 多个 session 只发你好

1. A/B/C 各发「你好」：Pulse 里三条原文 `你好`，session 不同。
2. 任一新会话第一轮就能看见这三条。
3. Agent 可以问「你连续在几个会话里只问好，是要开始做事还是误触？」
4. AutoDream 稍后可把三条收成 Work 一条 Open。未收之前，Pulse 原文已经够用。
5. 这些「你好」**不**写成 BML「用户喜欢打招呼」。

### 11.3 用户在 B 纠正「那不是任务」

1. 用户改 Work 或删 Open 卡片：直接生效。
2. Pulse 仍保留「说过你好」的近讯，除非用户删那条。
3. 不产生 Proposal，不写 BML。

### 11.4 长期事实不走 STM

「用户只要简洁 changelog」是 LTM，应 `memory_add` 进 BML（或将来的 USER 观察提案），不是写进 Work 当永久偏好。Work 最多留「本轮按简洁口吻改」这种**当前约束**，任务结束后删。

---

## 12. 失败、并发、寿命

- Pulse 追加失败：本轮仍应让模型看见「本条用户话」（它已在 CurrentUser / history）；后台重试进 Pulse。不得因为 Pulse 失败丢掉用户原话。
- Work 写入用 CAS：过期响应不得覆盖新 revision。失败保留上一份（`revisions/`）。
- 两个 session 同时说话：Pulse 按时间追加即可。Work 冲突：后写若 CAS 失败则重读再合并字段（Goals/Open/Next 分开），不能 last-write-wins 整文件。
- Session 结束：删 SessionCheckpoint，**不删** STM。删 STM 某胶囊，**不删** transcript / BML。
- Workspace 删除：STM 随 workspace 走，不泄漏到别的 workspace。
- BML 库损坏走 `DegradedMemoryProvider` 时，STM 仍应可读——这是独立目录的好处。

---

## 13. 和现有对象的边界（实施时对照）

| 现有对象 | 以后 | 与 STM |
| --- | --- | --- |
| `sessions/*.jsonl` | 保留，全文权威 | Capsule / Pulse 只存指针 |
| `canonical_checkpoint_v1` | 保留，本会话 compact | 禁止当 STM |
| `WorkingMemory` / SessionCheckpoint | 改名，session scratch | 可当 Work 的输入，不是权威 |
| BML records | 唯一 LTM | 只允许 Pointers 引用 |
| `memory_md` | 删除 | 禁止落点 |
| Persona 七文件 | 人格 | AutoDream 可提案，STM 不存人格正文 |
| Plan | 审批过的任务计划 | 不升格为 STM |

---

## 14. 本提案请求你拍板的点

请明确同意或改数字/改否，不要留成「实现者看着办」：

1. **注入面合成一份文件**（Pulse + Work 两节），胶囊仍分开、不注入。——**用户已同意方向**
2. **物理**：独立 `.laputa/stm/`，不进 BML sqlite。同意吗？
3. **注入**：`{NAME}.MD` 每轮必注入；胶囊只工具。——**用户已同意方向**
4. **实时**：用户发言立刻写 Pulse 节；空闲只写胶囊。同意吗？
5. **用户短句原文进 Pulse，助手全文不进 Pulse。** 同意吗？
6. **预算初值**：Pulse 1600 字 / Work 1600 字 / 单胶囊 800 字。要改数吗？
7. **Scope**：每 workspace 一份。对吗？
8. **cron/subagent 第一版只写 Pulse、不改 Work。** 对吗？
9. **注入文件名**：`STM.MD` / `ACTMEM.MD` / `STMEM.MD` / `ACTIVE.MD` / `FOCUS.MD` / `LIVE.MD` / `BOARD.MD`？本提案推 `STM.MD`。

下面这些即使本方案被接受，也仍留给 D2 细设计，不在这次拍板：

- Markdown 解析器、CAS 字节格式、具体 section 枚举名
- 精确插入 C1–C5 的 Rust 类型
- GUI 线框像素
- AutoDream 整理算法的阈值（连续几次问好算模式）
- STM → BML 的晋升规则（本提案默认：**第一版不做自动晋升**）

---

## 15. 验收（方案被批准并实施之后）

- A 聊完开 B，问刚才聊了啥：不靠 BML，不靠翻错 session，能答到要点。
- 三个 session 只发你好：第四个 session 第一轮就能提起这件事。
- BML 列表、`memory_search`、L1 索引里没有 STM 卡片。
- 删 STM 不影响 BML 和 transcript；session 结束不影响 STM。
- 投影超顶时 GUI 里的 Work 与模型看见的 Work 一致（权威侧淘汰）。
- 用户在 STM 工作区改一处，下一轮立刻生效且无 Approval。
- AutoDream 可以改 Work / 胶囊，不可以 `memory_add` 出这些内容。

---

## 16. 明确不在本提案里实施

用户批准之前：不改生产代码，不建 `.laputa/stm/`，不接线 ContextBuilder，不改 AutoDream worker。Research Gate / D0–D2 门禁仍有效。本文件只是完整方案稿。
