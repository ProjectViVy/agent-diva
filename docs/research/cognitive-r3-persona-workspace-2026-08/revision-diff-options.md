# R3 revision / Diff / CAS / 历史选项

- 状态：`Research Draft / Options Only`
- 日期：2026-08-13
- 性质：比较 Research Hold 选项与静态实验；**不宣布获胜方案**
- 约束来源：Persona 决策 P4–P6、P11–P12；盘点见
  [`persona-authority-inventory.md`](./persona-authority-inventory.md)
- 「高冲突」不是否决，只是 D1 必须显式消化的代价

产品已冻结、本包不当成选项的条目：

- 每次真实成功变化追加完整 Markdown 快照 + 文本 Diff；不自动裁剪
- 完整历史不进 Prompt
- no-op 不建 revision；拒绝请求只留审计
- 载入历史 = 本地草稿；保存 = 新头；禁止原地改写历史
- 第一版无三方合并、无逐段接受、无 approve/apply 分裂
- 保存带显式 base revision；冲突保留草稿；待审 base 失配 → `stale`

## 1. 今天实际有什么（静态实验）

| 实验 | 结果 | 标签 |
| --- | --- | --- |
| `rg content_version` | 定义于 `frozen_core.rs:188`；Manager 投影展示；**无写路径比较** | 源码事实 |
| `rg base_revision` | Persona/WORLD 写路径 **零匹配** | 实验观察 |
| `rg if_match` | 同上 | 实验观察 |
| section write payload | `{ content, actor?, summary? }` | 源码事实 `handlers/laputa.rs:134-138` |
| `LaputaSection.version` | `schema_version` 字符串，不是内容哈希 | 源码事实 `service.rs:562` |
| `unified_diff` | 全部删行 + 全部加行，假 unified 头 | 源码事实 `proposals.rs:729-748` |
| changelog 回滚 | 30 天；`expected_current` 比文件；Typed 可不写权威；`stale` 恒写 `false` | 源码事实 `service.rs:612-672` |
| `LaputaLock` | `proposals` / `events` / suppression / feedback / migration；**不锁** section 或 WORLD.MD | 源码事实 |
| 五文件事务 | 无。`initialize_sections` 与 `initialize_dir` 逐文件 `atomic_write` | 源码事实 |
| BML `record_revision` | Memory 行 CAS 对照物；跨进程仍无 flock（R2） | 源码事实；对照 |

结论（实验观察）：Persona/WORLD **没有**文档 CAS、没有永久 revision store、没有真实
逐行 Diff 引擎、没有五权威原子提交。最接近的三件东西名字会骗人：

| 本包称呼 | 代码对象 | 不是 |
| --- | --- | --- |
| DisplayDigest | `content_version` SHA-256 | 写前置条件 |
| GovernanceChangelog | `ChangelogRecord` + 30 天 rollback | 永久文档历史 |
| NaivePatchDump | `unified_diff` | 对齐文本 Diff |

## 2. 当前权威物理形态（Hold）

| 选项 | 要点 | 与现状冲突 | 失败模式 |
| --- | --- | --- | --- |
| A. 每对象一个 `.md` | 贴近 P1/P8 推荐名；人类可打开；`atomic_write` 已存在 | 必须停用 `sections/*.json` 与 `read_section` JSON 解析；五文件原子要另做 | 半写；编辑器与进程双开覆盖 |
| B. 单一 sqlite 文档表 | 五文件天然一事务；易做 CAS 列 | 新恢复面；与 BML 同库会诱使混权威；抽层门禁要扩 | 库坏五份全挂；迁移诱惑 |
| C. 文件正文 + sidecar 元数据 | `.md` 给人看，`.rev.json` 存 revision | 双文件仍要原子；sidecar 丢失则 CAS 瞎 | 元数据与正文分叉 |
| D. 继续 JSON section 只把 Value 改成 string | 改动面小 | **高冲突** P1/P8：禁止 JSON 权威与 `.laputa/sections/` | 假装 Markdown 仍走旧路径 |

建议（非批准）：D1 在 A/B/C 中论证；D 只允许作为否决对照。本包不选。
决策记录推荐文件名 `IDENTITY.md` 等，但**最终目录名仍是 Hold**。

## 3. revision 身份（Hold）

| 选项 | 要点 | 冲突 | 失败 |
| --- | --- | --- | --- |
| A. 纯内容 SHA（现状 `content_version` 形态） | 同文同 id；天然 no-op | 无法区分「同一正文两次合法保存」；时钟回拨无关但审计弱 | 历史列表两条看起来像一条 |
| B. 每文档单调序号 | 人类可读 v1/v2 | 多进程抢号；要锁或 sqlite | 序号洞、双头 |
| C. 序号 + 内容哈希 | 满足 CAS 与去重 | 多一个字段 | 实现稍重 |
| D. 内容寻址 blob 库（git-like） | 相同正文共享对象 | 新子系统；目录 Hold 更大 | GC 诱惑与 P12「不裁剪」对撞 |

产品要求历史记录 revision、完整快照、相对上一版 Diff、actor、时间、原因、
base revision。A 单独不够表达「恢复旧文再保存」应产生**新头**（内容可能与旧版
相同哈希）。推断：纯内容 SHA 作**主键**会与「新头」冲突；作**校验**可以。

建议（非批准）：D1 把「身份」和「内容校验」拆开讨论，不要把 DisplayDigest 直接
升成主键。

## 4. 快照策略（Hold）

产品要求每次真实变化有**完整** Markdown 快照。选项只谈怎么存：

| 选项 | 要点 | 冲突 | 失败 |
| --- | --- | --- | --- |
| A. 每版本完整正文 | 实现简单；符合「完整快照」字面 | 磁盘随编辑线性涨 | 大文档 + 永久保留体积（R4 抽样） |
| B. 只存 diff 链 + 周期全量 | 省空间 | 与「完整快照」验收措辞打架；读历史要重放 | 链中间损坏丢一段轨迹 |
| C. 完整快照 + 内容寻址去重 | 同文共享 blob，元数据仍每条 | 见 §3 D | 实现复杂；仍要证明「不裁剪」 |

B 若只存 diff、读时重放，测试必须证明任意 revision 能还原字节级正文。
产品验收写的是「完整快照」，B 是高解释成本，不是自动非法。

## 5. 文本 Diff（Hold）

现状 `unified_diff` 不能当产品 Diff：Unicode 行、空文档有特判，但无对齐、
无改行识别、固定单一 hunk。

| 选项 | 要点 | 冲突 | 失败 |
| --- | --- | --- | --- |
| A. 行级 Myers/LCS，存 unified 文本 | 可测；GUI 高亮解析同一格式 | 要引入/自写 diff crate | 超长行、无换行文档退化 |
| B. 词级 / 字级 | 中文人格文档更可读 | 库更重；历史体积更大 | 性能；不稳定分词 |
| C. 只存 after，浏览时现算 Diff | 省存储 | 算法升级会改变「历史里的 Diff」 | 旧版视觉漂移 |
| D. 继续 JSON patch / naive dump | 零工作 | **高冲突** P5/P7 | `[object Object]` 类症状回流 |

产品 P5：只读 before/after、同步滚动、逐行增删改高亮。第一版不要词级也能过
验收。建议（非批准）：D1 把「存储的 Diff」和「GUI 渲染的 Diff」分开；C 合法，
但验收必须钉死算法版本。

## 6. CAS / stale（约束 + 实现选项）

约束：用户保存带 base；过期拒绝且保留草稿。待审请求创建时钉死 base；之后人类
直存 → 请求 `stale`，禁接受。

| 选项 | 要点 | 失败 |
| --- | --- | --- |
| A. If-Match 内容哈希 | 与 DisplayDigest 同形 | 同文两次保存要靠别的字段防 no-op |
| B. If-Match 单调 revision | 清晰 | 丢序号就全体 409 |
| C. 哈希 + 序号都传 | 冗余但可诊断 | 客户端必须都存 |
| D. 无 CAS，last-write-wins | **高冲突** P4/P5 | 旧建议覆盖新人类编辑 |

stale 检测是服务端职责：接受时再比一次当前头与 `base_revision`，不能只信 GUI。
现状没有任何一边做这件事。

409 / stale 的稳定 reason code 必须进 Manager/Tauri 信封（决策已要求）。本包
不拟定 DTO 字段名。

## 7. 回滚语义（实现，不是产品重开）

产品：禁止把历史拨回去或抹中间版本。实现选项：

| 选项 | 要点 | 冲突 |
| --- | --- | --- |
| A. 复制旧快照到本地草稿（GUI only） | 完全符合 P6 | 需要中央历史态；今日 HistoryModal 做不到 |
| B. 服务端 `restore` API 仍只返回正文、不写权威 | 便于 CLI | 须防被做成原地写 |
| C. 调用现 `rollback_changelog` | **高冲突**：30 天窗、改权威、治理反操作 | 会删除轨迹或写回旧 JSON |
| D. git revert / 原地改文件 | **高冲突** P12 | 历史可变 |

建议（非批准）：把 changelog rollback 留在 DELETE/治理清理面，不要复用。

## 8. 历史物理分离（Hold）

| 选项 | 要点 | 冲突 | 失败 |
| --- | --- | --- | --- |
| A. 当前头一个 `.md` + `revisions/` 每版本一文件 | 人类可备份；与权威分离 | 五文档 × 永久版本 = 文件数 | 半写版本文件 |
| B. 同库历史表（可与 §2 B 合一） | 查询/分页简单 | 见 sqlite 代价 | 单库损坏 |
| C. 改造 `changelog/<id>.json` | 已有 before/after/diff | **高冲突**：绑 proposal、30 天、page 100、naive diff、Persona 混 Memory | 治理清理会误删人格史 |
| D. git 仓库当历史 | 现成工具 | 新依赖；Windows 文件锁；与「不裁剪」要禁 gc | 用户误操作 rebase |

## 9. 五文件原子初始化（Hold）

产品：一次提交创建五份权威 + 各自首个历史；任一点失败不得 `ready`。

| 选项 | 要点 | 失败 |
| --- | --- | --- |
| A.  staging 目录写齐后 rename 到权威目录 | 沿用 `atomic_write` 思路扩到目录 | Windows 上非空目录 rename 不原子；需空目录交换或单文件 manifest |
| B. sqlite 一事务插五头 + 五历史 | 真正原子 | 与 §2 绑死 |
| C. write + manifest（五路径+哈希写完才标 ready） | 崩溃可识别 `incomplete` | 窗口期内读者要认 incomplete |
| D. 逐文件 `atomic_write` 像今天 seed | **高冲突** P11 | 半初始化；正是今天要删的行为 |

现状 `initialize_sections` 循环写四个 `null`，`initialize_dir` 再写 WORLD，
中间崩溃会留下部分文件 → 产品定义的 `incomplete`，但今天没有修复入口。

## 10. WORLD 是否共用 revision store（Hold）

| 选项 | 要点 | 冲突 |
| --- | --- | --- |
| A. 与四份 Persona 共用文档引擎 | 一次初始化、同一 CAS/历史 UI | WORLD 今天是 claim 文件 + 独立治理队列；`project()` 语义要另挂 |
| B. WORLD 继续独立文件 + ledger | 少动 consolidation | 五权威原子更难；GUI 两套历史 |
| C. 混合：WORLD 正文走文档引擎，claim 投影仍用 `WorldStore` | 人类看到一份 Markdown 历史；运行时仍按 claim 保护 | 双解析器；序列化必须往返稳定 |
| D. 保留 WorldGovernance pending 当「WORLD 待审」 | 状态机是 Applied/Rejected，不是 Persona 四态 | 借 WORLD 复活通用治理 |

产品：WORLD 参加首次引导，但是独立 claim 权威。A/C 都合法。D 高冲突于
「不得借 Diff/文档基础设施复活通用 Proposal」。

`WorldStore::project` 是否进 Prompt 是**另一个** Hold（装配），不是 store 形态。
R2 已写 Frozen Core 在 prefix 第二段；WORLD 全文被禁。D1 若接线 `project()`，
必须另选插入点并遵守预算，不能塞进 Frozen Core 整包。

## 11. 并发（Hold）

对照 R2：BML `put` 有进程内 Mutex + SQLite WAL + 记录 CAS；跨进程无写锁。
Persona 更弱：写路径甚至没有文档 CAS。

| 选项 | 要点 | 失败 |
| --- | --- | --- |
| A. 纯 CAS，无跨进程锁 | 实现窄；符合「保存带 base」 | 两 GUI 同时编，后保存 409；草稿靠客户端 |
| B. `LaputaLock` 扩到文档写 | 已有锁文件原语 | 5s timeout / 5min stale；跨进程卡死 |
| C. 单 writer 队列 | 简单 | 用户直存 vs Agent 变更请求仍是双 writer |
| D. 与 BML 同事务 | 只有 §2 B 才有意义 | 人格写失败拖垮 Memory |

产品最低：冲突不得用服务端内容覆盖草稿。任何选项都要有稳定错误码。
建议（非批准）：A 是与 P4 最贴的最小集；B 可作多进程补充，不能替代 CAS。

## 12. no-op / 拒绝 / Agent 请求（约束落地）

不是选项：

- 保存正文与当前头相同 → 200/幂等，不追加 revision
- 拒绝 `PersonaChangeRequest` → 只留请求审计，不写文档历史
- 接受 = 一次原子：校验 base + 写权威 + 追加历史 + 终结请求

现状全部落空：每次保存都新建 PendingReview 提案；拒绝走 Governance deny；
没有 Persona 四态。

Agent 何时创建请求、一文档几个 pending：产品 Hold，本包不填默认值。
D1 不得让实现者用「复用 EvolutionProposal」当捷径。

## 13. 建议（非批准）汇总

D1 应显式论证、且本包**未选**的组合空间：

- 物理：A/B/C（每文件 / sqlite / sidecar）
- revision 身份：不要只用纯内容 SHA 当主键
- Diff：行级可测算法；不要 naive dump
- CAS：必须有；stale 在服务端再检查
- 历史：不要改造 changelog
- 初始化：不要逐文件 seed
- WORLD store：与 Persona 共用或混合均可，但不要把 WorldGovernance 当通用提案
- 并发：CAS 是底线

这些句子是研究标记，不是架构批准。
