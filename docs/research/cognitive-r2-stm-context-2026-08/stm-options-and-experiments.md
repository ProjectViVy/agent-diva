# R2 STM 选项与实验

- 状态：`Research Draft / Options Only`
- 日期：2026-08-13
- 性质：比较 Research Hold 选项与静态实验；**不宣布获胜方案**
- Hold 来源：`../stm-cross-session-clean-break-2026-08/decision-record.md` §分层与装配

每个 Hold 列出可检验选项、与当前 C1–C5 / BML / Session 的冲突、失败模式。
「高冲突」不是否决，只是 D2 必须显式消化的代价。

## 1. 物理权威（Hold 1）

| 选项 | 要点 | 与现状冲突 | 失败模式 |
| --- | --- | --- | --- |
| A. 独立 Markdown 文件 | 人类可 diff；贴近历史 `05 MEMORY.MD` 语义 | S2 删除的是 `memory_md` section 链路，不是「一切 Markdown」。但仍易被当成第二 LTM；无 CAS；多进程无锁 | 半写文件、双打开覆盖 |
| B. BML 新 `MemoryRecordKind`（`session_id=None`） | 复用 CAS / FTS / 列表 API | S4 禁止一个 kind 表达两种寿命；S1 STM 不是 LTM；`AppliedAuthority` 会被 L1 吞进 prefix | STM 进 L1；session GC 若误判 `session_id` 会误删或留不下 |
| C. `memory.sqlite3` 独立表 | 与 LTM 同文件、不同 schema / GC | BML 边界门禁要扩 allowlist；同库降级会拖垮 STM；抽层决策 D 不新开 memory crate，但表仍在 laputa | 事务耦合、迁移诱惑 |
| D. 独立 sqlite 文件 | 与 BML / governance / planning 并列 | 新恢复面；多库一致性；Garden 未实现，少一个现成 facade | 文件缺失、锁、备份遗漏 |
| E. 复用 Plan store | 已有 goal / open_questions | **高冲突**：审批、`EphemeralPlanRegistry` 重启丢、`active_plan` 单例、启动删旧 `planning.db*` | 把 STM 绑上 Approval |
| F. 升格现有 `WorkingMemory` | 改 GC、改 `session_id=None` | **高冲突**：S4 明文禁止；Reset/Delete 语义、GUI kind、L1 过滤全要翻 | 一个 enum 两种寿命 |

静态实验 E1 / E8：`memory_md` 仍是活 section，但 S2 已判 DELETE；沿用它会与 clean break 冲突。
实验观察：无第三份活动集权威。

建议（非批准）：D2 应在 A/C/D 中论证，并把 B/E/F 的冲突写进 ADR 否决或极窄例外。本包不选。

## 2. Scope（Hold 2）

现成 scope 列：`MemoryScope { tenant_id, workspace_id, session_id }`。
活身份是 `{channel}:{chat_id}`，不是 user，也不是 workspace。

| 选项 | 含义 | 冲突 |
| --- | --- | --- |
| Workspace 一份 STM | 新 GUI chat、CLI、Slack 看到同一活动集 | 多 channel 并发写；cron / 子代理公告打回父 session 时会争用 |
| Profile / 用户一份 | 跨 workspace 持续 | 当前无 profile 级 store；BML 以 workspace 为租户 |
| Agent / Mask 一份 | 换人格换 STM | Frozen Core 已是 session 冻结；再拆一层易混 |
| 每 session 一份再投影 | 接近现 WM | **不满足** S3「新 session 必须看到同一个 STM」 |
| 每 channel 一份 + 合成投影 | 减少写冲突 | 合成规则本身是新 Hold |

S3 要求跨 session 持续，因此「每 session 一份」只能当输入，不能当权威。

## 3. 并发合并（Hold 2 / 9）

现状（源码事实）：

- BML `put`：进程内 `tokio::Mutex` + SQLite WAL + 5s busy + 记录 CAS
- 跨进程：**无** BML 写锁（`LaputaLock` 管提案/迁移，不管 `put`）
- Session JSONL：last rename wins，无 flock
- 两 session 写 WM：不同 `memory_id`，不合并

| 选项 | 要点 | 失败 |
| --- | --- | --- |
| Last-write-wins + CAS | 复用 BML revision | 过期响应覆盖；无字段合并 |
| 字段级 merge（目标/回路/下一步分开） | 降低互踩 | 需要 schema；冲突字段仍要规则 |
| 单 writer 队列（AgentLoop 串行） | 实现简单 | GUI 用户修正与 Agent 自动更新仍可能双 writer；多进程无效 |
| 分片再投影 | channel 本地 + 合成 | 合成延迟、双真源风险 |

产品最低测试已要求：自动更新失败保留上一份；过期响应不得覆盖新 revision。
任何选项都必须满足这两条，不能只靠「最后写入赢」。

## 4. 自动触发（Hold 3）

现状没有 STM 触发器。接近的 cadence：

| 现成事件 | 今天做什么 | 若拿来驱动 STM |
| --- | --- | --- |
| 模型调 `update_working_checkpoint` | 写 SessionCheckpoint | 无 Action-Verified；证据数组空 |
| Auto compact 成功 | 换 CanonicalCheckpoint | 有损摘要，不宜当唯一输入 |
| Plan phase 变化 | registry / sqlite | 审批域，且 draft 不跨重启 |
| `on_session_end` | **删除** WM | 与 S3 持续相反 |
| Cron inbound | 新 turn | 可能写错 session（CLI `cli:direct`） |
| GA `turn % 7` 催 checkpoint | Diva **未抄** | 软催，GA 自己也不强制 |
| Heartbeat | **未接线** | 不能当现成触发器 |

选项：turn end 启发式 / compact 成功后抽取 / 工具证据门 / 用户显式 / 定时 /
不自动（只手写）。必须保留「不自动」作为合法选项。

Action-Verified：R1 已证 GA 也只是提示词纪律。Diva L0 三句常驻 prefix，**无**代码门。
STM 日常写 S5 不走审批，因此不能把 governed-apply 当主路径。晋升到 BML 才需要证据门槛
（Hold 10）。

## 5. Schema（Hold 4）

S3 已冻结产品字段，不是物理 schema：

- 当前目标
- 开放回路 / 未完成事项
- 已确认下一步
- 当前有效约束与关键决定
- Skill/SOP **引用**
- 指向 BML / WORLD / 会话 / artifact / 外部证据的最小指针
- 最近更新时间、来源、自动化状态

选项只涉及载体：单 Markdown 文档 / 多卡片记录 / 单行 JSON / 多表。
本包不发明列类型。

现成「像 STM」但不是 STM 的结构（约束文 §1、§6）：Plan.goal / open_questions、
checkpoint 固定节、WM `key_info`、`todos.jsonl`、`HEARTBEAT.md`、Laputa Commitment。
D2 可以**读取**它们当输入，不能把其中任何一个升格为权威。

## 6. 预算与淘汰（Hold 5）

| 选项 | 对照 | 冲突 |
| --- | --- | --- |
| 学 GA L1：≤30 行 / &lt;1k token | 那是索引，不是活动集 | 同名 L1 陷阱 |
| 学 WM 层：3%–4% 且 Never evict | 活动集变大时会挤 ActiveTail | Never 与「有界收敛」张力 |
| 学 Garden WorkingSet：cards≤50 | sibling 参考，非正式合同 | 条数 ≠ token |
| 学 checkpoint：8000 字替换 | 有损；不能当活动权威 | S4 |
| 完成/替代/到期淘汰，不靠 token | 需要状态机 | Hold 本身 |

S3：随任务完成、失效、替代和预算压力自动收敛，不能长成第二个 LTM。
`EvictionPolicy::Never` 与「必须收敛」同时成立时，淘汰必须发生在**权威侧**（删卡片），
而不是装配侧静默 Drop——否则模型与 GUI 看到的 STM 会分叉。

## 7. 历史 / 撤销 / no-op（Hold 6）

对照：Persona 决策要求**永久**完整快照；STM 是有界工作集，历史策略可以更短。

| 选项 | 要点 |
| --- | --- |
| 不保留历史，只保留当前 revision + CAS | 撤销靠「用户再改回去」 |
| 有界环形快照（N 份） | 满足「失败保留上一份」 |
| append-only 全文 | 易变成第二个 Persona 历史 |
| 软删除卡片 + 撤销 | 贴近 Memory CRUD 误操作保护 |

No-op：内容 digest 不变则不升 revision，避免无意义 cache break / 历史噪声。

现状 SessionCheckpoint **没有**历史：同 id 覆盖写，`supersedes` 空。失败时
`checkpoint_write` 返回 `Failed`，旧行仍在——这是「失败保留上一份」的现成行为，
可作 STM 参考，不是 STM 实现。

## 8. Layer 1 装配位置（Hold 7）

此处 Layer 1 = **产品 STM 的有界 Prompt 投影**，不是 GA insight，也不是 Diva L1 索引。

| 选项 | cache | 新 session 首轮 | compact | 清空风险 |
| --- | --- | --- | --- | --- |
| prefix 内新 SessionStable section | 每次自动更新 break | 稳定可见 | 不受 compact 影响 | 低 |
| 并入 `MemoryPolicyAndIndex` | 同上，且与 LTM 索引缠死 | 稳定可见 | 同上 | 中（L1 过滤规则） |
| prefix 后、history 前独立区（第四顶层 region） | 可不进 prefix hash | 每 turn 重读 | 需规定是否计入 checkpoint 预算 | 低 |
| 并入 `WorkingMemory` 槽 | 不破 prefix | 动态区可读 | 与 session GC 同槽 | **高** |
| 写入 CanonicalCheckpoint body | 不破 prefix | 依赖该 session 是否已 compact | 权威被有损替换 | **高** |
| 仅 GUI，不进 Prompt | — | 模型看不见活动集 | — | 违反 S3「活动上下文」 |

约束文 §8 已列否定倾向（并入 WM / checkpoint / Plan / `memory_md`）。本表仍全部保留，
供 D2 论证。

## 9. 与其它块的优先级（Hold 8）

不得合并权威。只规定「同一事实出现两次时模型应信谁」——建议级，见约束文 §8.9。

负向不变量（已有 + STM 必须继承）：

- 完整 WORLD / MEMRULES / 报告 / 退役人格文件不进默认 Prompt
- SessionCheckpoint 不进 BmlStartupIndex
- CanonicalCheckpoint 历史版本不重新注入
- STM 不整体注入 BML / transcript / WORLD

## 10. 晋升到 BML / Skill（Hold 10）

现状：

| 路径 | 立即写 BML？ | 证据 |
| --- | --- | --- |
| `memory_add` | 是，LongTerm | 无强制 evidence |
| `memory_update` / `memory_remove` | 否，走提案 | 旧治理链 |
| `memory_distill` | 写 `skills/<name>/SKILL.md` | 可选 evidence 字段 |
| `checkpoint_write` | 是，但 kind=WorkingMemory | `evidence_refs=[]` |
| AutoDream | 提案，不直接 `memory_add` | R1 切片 |

选项：

1. **不自动晋升**（必须保留）。STM 过期或完成后丢弃或归档，人/Agent 显式 `memory_add`
2. 证据门：工具结果或用户原话才能 `memory_add`；无门则拒绝
3. 蒸馏到 Skill：只允许引用已存在 Skill，或走未来 Evolution（D3），不在 STM 写 How-to

证明「不会把易变状态固化」的最低实验：STM 更新路径零调用 `put` LongTerm / `put_governed` /
`memory_distill`；晋升走独立、可审计入口。

## 11. GUI（Hold 11 / S6）

产品已冻结：Memory 页右上角专用入口 + 完整工作区；不是 tooltip，不是 `working_memory`
筛选值。Persona / Evolution / Notebook 不承载 STM。

本包只登记信息需求，不画布局：

- 当前目标、开放回路、下一步、约束与决定
- Skill/SOP 引用、证据指针、来源 session
- 最近自动更新时间、自动化状态、容量/预算
- 更新历史（若 Hold 6 选择保留）
- 用户动作：看来源、修正、完成、移除、看历史；**不**产生 Approval

实验观察：`MemoryView.vue` 仅有 kind 筛选含 `working_memory`；无 STM 入口文案或路由。

## 12. 静态实验记录

| ID | 做法 | 结果 | 等级 |
| --- | --- | --- | --- |
| E1 | `rg struct Stm\|enum Stm\|fn stm_` 于 `*.rs,*.vue,*.ts` | 零匹配；产品 STM 不存在 | 实验观察 |
| E1b | `rg run_startup_gc` | 仅 `typed_provider.rs:204` 定义 | 实验观察 |
| E2 | 读 `CONTEXT_SECTION_ORDER` + `c1a_t2` | WM/Recall 变化不得改 prefix | 源码事实 |
| E3 | 并排 GA `get_global_memory` / Diva `render_l1_index_block` / Garden Layer 1 | 三套 L1 同名不同物；见 README | 源码事实 + R1 |
| E4 | 列 `{channel}:{chat_id}`、subagent、cron 重映射 | GUI cron 隔离；CLI cron 共享 `cli:direct` | 源码事实 |
| E5 | 跟踪 `memory_add` / distill / `memory_md` / WM put | 无跨会话活动集；WM 不进 L1；distill 写 Skill 文件 | 源码事实 |
| E6 | `checkpoint_write` 失败返回 Failed；旧行仍在 | 「失败保留上一份」在 WM 上成立；Reset/Delete 不调 GC | 源码事实 |
| E7 | MemoryView kind 列表 | 有 `working_memory`，无 STM 工作区 | 实验观察 |
| E8 | `MemoryMd` 仍活 vs S2 DELETE | 沿用 `memory_md` 与 clean break 冲突 | 提交事实 + 源码事实 |
| E9/E10 | GA / Diva 活体 | **未跑**（无隔离密钥；桌面跨 session 留给验收） | 开放项 |
| E11 | 合成载荷塞进各装配槽 | 未跑动态预算；由层表静态推演 | 推断 |
| E12 | 新 kind / 新表 vs `bml-boundary-check` | 治理模块不得直调 put/GC；STM 日常写若进 BML 只能走 typed_provider 允许面 | 源码事实 |

未跑 E9–E11 不阻断本包交付；阻断的是把未跑实验写成「已验证行为」。

## 13. 给 D2 的未决清单（必须显式选择）

1. 物理权威 A/C/D（或带否决理由的 B/E/F）
2. scope：workspace vs profile vs 合成投影
3. 并发：CAS-only vs 字段 merge vs 单 writer vs 分片
4. 自动触发集合，以及「不自动」是否保留为默认
5. 装配位置（§8 六选项之一）
6. 历史深度
7. 晋升：默认不自动，还是证据门
8. subagent / cron 的读/写权限
9. GUI 是否第一版就做历史

本文件任何「建议」行都不是上述选择。
