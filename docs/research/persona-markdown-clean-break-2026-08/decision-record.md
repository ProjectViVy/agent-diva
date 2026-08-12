# Persona Markdown 权威与编辑体验 Clean Break 决策

- 状态：`Approved Direction / Implementation Pending`
- 记录日期：2026-08-13
- 性质：产品与架构边界决策；不是完整实施计划

## 问题判断

当前 Persona 页面看似只存在编辑器呈现问题，实际错误贯穿完整领域模型：

- Frozen Core 人格 section 物理存储为 `.laputa/sections/*.json`；
- `LaputaSection.content` 使用 `serde_json::Value`；
- 用户编辑必须通过 JSON parse/schema 校验；
- 人格 Proposal 保存 JSON patch；
- Frozen Core 会话快照再次把 section 内容序列化成 JSON；
- GUI 暴露 `JSON.stringify`、`JSON.parse`、“格式化 JSON”和 JSON 错误；
- Persona 右侧同时承载 proposal、governance、批准、拒绝、应用与回滚生命周期。

这不是局部 UI 缺陷，而是人格正文被错误建模为 JSON，并把审批系统混入人格管理。

项目早期设计已明确要求 Markdown 编辑器，并特别禁止把 JSON 校验硬塞进 Markdown：
`docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md`。后续 Frozen Core
JSON 重构偏离了该产品语义。本次恢复 Markdown 方向，但不恢复早期 14-section、Memory
混入人格、旧文件映射或隐式 proposal/apply 等旧设计。

## 已确认决策

### P1：人格正文的唯一权威格式是 Markdown

- Identity、Relationship、Commitment、Preferences 继续作为四个独立人格对象。
- 每个对象拥有一个 Markdown 正文；读取、编辑、版本 hash、Diff、历史、回滚、
  Frozen Core 捕获和 Prompt 投影均以 Markdown 字符串为准。
- 人格领域模型不得再用 `serde_json::Value`、JSON object、JSON patch 或
  `content_type: json` 表达正文。
- 空内容使用空 Markdown/TBD 状态，不使用 `null`、`{}` 或占位 JSON。

### P2：Persona 页面是人格文档工作区，不是安全审批中心

- Persona 页面只负责查看、编辑、预览、比较和管理人格文档历史。
- 用户手动编辑直接保存到人格权威，同时生成 changelog/audit；不创建一个还需要同一
  用户再次批准的 Proposal。
- 删除 Persona 页面中的 Governance Ledger、通用 Proposal、仅批准、暂缓、独立应用和
  proposal patch 编辑入口。
- Agent 或系统提出的人格文本修改进入 Persona 专属“变更审查”，由用户在文档工作区
  接受或拒绝。这里的接受/拒绝是内容审阅，不是工具权限或风险授权。
- 危险工具执行继续由聊天页右侧统一 Approval Center 负责；人格页面不复制审批入口。
- Persona 变更审查不得进入聊天页 Approval Center，也不得复用 Evolution、Memory 或
  Sandbox 的治理状态机；用户对自己人格文档的直接编辑不受待审变更阻塞。

### P3：工作区只保留左侧导航和一个中央文档工作区

删除永久右侧生命周期栏。左侧继续作为四个人格文件导航；中央区域在三个互斥状态之间
切换：

```text
当前文档 | 待审变更 (n) | 历史
```

- 左侧只显示四个人格文档、更新时间、未保存草稿标记和待审数量，不混入普通 Memory、
  AutoDream 或 Evolution。
- 三个中央状态不能同时挤在多栏页面中；每个状态只呈现完成当前任务所需的主操作。
- 宽屏允许中央状态内部使用双栏；窄屏退化为明确的单工作区标签或顺序视图。

### P4：当前文档状态使用“Markdown 源码 + 人类预览”

- 左侧为极简 VS Code 风格 Markdown 编辑器，至少支持行号、语法高亮、当前行、查找、
  撤销/重做、自动换行、滚动、`Ctrl/Cmd+S` 和明确 dirty 状态。
- 右侧实时渲染当前草稿的人类可读预览；预览可以收起，收起后编辑器占满中央区域。
- 用户保存时携带显式 base revision/CAS；成功后正文原子成为当前权威并追加不可变历史。
- 保存失败或 revision 冲突必须保留草稿、当前文件选择和编辑位置，不得回填服务端内容
  覆盖用户输入。
- 切换文件或载入历史会丢弃未保存草稿时，必须在动作边界明确提示。

### P5：待审变更状态是只读的 Markdown 内容审查

- 左框显示变更请求创建时的准确 base Markdown，右框显示 proposed Markdown；两侧只读、
  同步滚动，并使用真实的逐行文本 Diff 高亮新增、删除和修改。
- 主动作只有“接受变更”和“拒绝变更”。接受必须在一次原子操作中校验 base revision、
  写入人格权威、追加历史并终结请求，不再拆成 approve 与 apply 两步。
- 拒绝只终结请求，不修改人格权威。
- 第一版不提供仅批准、暂缓、逐段接受、提案内编辑或自动三方合并。
- 如果用户在请求产生后直接保存了当前文档，base revision 不再匹配，请求进入 `stale`；
  UI 必须禁用接受并要求重新生成或显式重建变更，不得让旧建议覆盖新的人类编辑。
- Persona 内容审查的最小领域状态为 `pending | accepted | rejected | stale`，不得映射回
  通用 Governance lifecycle。

### P6：历史状态区分不可变版本与当前草稿

- 历史记录作为中央工作区的独立状态，不放入永久右侧栏。
- “当前版本”和每个历史版本必须明确标识；历史记录只能查看，不能原位编辑。
- “载入到编辑器”只把选定历史正文复制到本地草稿，并标记为未保存；不会立即移动
  当前版本、写入权威或删除后续历史。
- 用户随后执行“保存为当前版本”才会写入人格权威，并形成一条新的历史记录。
- 如果编辑器已经存在未保存草稿，载入历史前必须提示本地草稿将被替换。

### P7：采用成熟编辑器与真实文本 Diff，不手写 textarea 伪装

- 实施前评估 CodeMirror 6 与当前 Vue/Vite/Tauri 依赖和包体；默认优先 CodeMirror 6，
  完整 Monaco 只有在确需语言服务时才考虑。
- 现有 `markdown-it` 可用于预览，必须保持原始 HTML 禁用，并核查链接协议和外部资源
  边界。
- 不通过给普通 textarea 增加行号背景或 CSS 高亮声称已实现代码编辑器。

### P8：破坏性 Clean Break，不做任何人格旧格式兼容

- 删除 `.laputa/sections/identity.json`、`relationship.json`、`commitment.json`、
  `preferences.json` 作为人格权威的能力。
- 删除旧 `SOUL.md`、`IDENTITY.md`、`USER.md` 等多源合并、迁移、legacy archive 导入、
  自动转换和 fallback。
- 新版本不读取、不转换、不探测旧人格 JSON 或旧人格 Markdown 文件。
- 不提供双读、双写、启动迁移、兼容 DTO、旧状态映射或隐藏恢复路径。
- 破坏性实施前必须从删除前的已验证提交建立保护性分支；保护分支只保存历史，不进入
  runtime，也不成为 fallback。
- 旧用户数据如何人工备份由发布说明明确，但产品不承担自动导入。

### P9：首次启动只初始化 Laputa 核心权威

首次引导不是普通 Memory 收集，也不是 Agent 在聊天中自行触发的一组 Proposal。它只负责
建立四份 Persona Markdown 权威与一份 WORLD 权威：

| 权威 | 首次引导要回答的问题 | 初始化语义 |
| --- | --- | --- |
| Identity | Agent 是谁、叫什么、具有什么职责、性格和表达方式？ | Agent 的初始自我定义 |
| Relationship | 用户是谁、如何称呼、双方希望建立什么关系？ | Agent 对用户及双方关系的初始理解 |
| Commitment | 初次相遇后 Agent 承诺什么、永远不越过哪些边界？ | 双方的初始契约与红线 |
| Preferences | 用户希望 Agent 朝什么方向努力，有哪些长期要求？ | 长期偏好与成长方向 |
| WORLD | 用户目前处于怎样的工作、生活或旅行环境？ | 独立的可行动环境认知 |

- Agent 对用户的初次看法属于 Relationship；Commitment 不是“第一印象”，而是承诺、
  约束和红线。Agent 不得自行解除 Commitment，但用户可以通过后续显式编辑修订当前版本。
- “不可变”有两个精确含义：每一个已经形成的历史版本永不改写；Frozen Core 在同一会话
  内冻结，当前权威的后续修改从下一会话生效。当前 Persona 文档本身不是永久锁死。
- WORLD 参加同一次首次引导和提交，但仍是独立的 claim 型权威；不得作为第五个 Frozen
  Core 整体注入 Prompt。

### P10：首次引导只由五份权威的物理存在状态触发

服务端以四份 Persona authority 文件和 `WORLD.MD` 的真实存在/有效性为唯一状态源：

```text
五份权威全部不存在 -> uninitialized -> 显示一次首次引导
五份权威全部存在且有效 -> ready -> 永不再显示首次引导
部分存在、空文件或内容损坏 -> incomplete -> 进入修复，不重跑首次引导
```

- 首次引导只能在五份权威全部不存在时出现；不能以“内容为空”“任意 section 有内容”、
  `localStorage`、聊天 session、配置向导完成标记或 Proposal 数量判断。
- 空文件表示已存在但不完整，不表示全新用户。不得借首次引导覆盖它。
- 实施时删除四个空 `null` Persona 文件和空壳 WORLD 的启动预种子；否则物理缺失条件永远
  不会成立。目录可以预创建，权威文件不能预创建。
- 部分写入或损坏是恢复问题。UI 应保留已有内容并引导进入 Persona/WORLD 修复入口，不能
  伪装成全新初始化，也不能自动补写默认正文。

### P11：初始化直接原子写入，不经过任何审批

- 用户完成引导后，只执行一次“完成初始化”。服务端在一个原子提交边界内创建五份权威
  和各自的首个历史版本；任一验证、写入或历史落盘失败，整体不得进入 `ready`。
- 初始化不创建 EvolutionProposal、PersonaChangeRequest、Memory proposal 或 WORLD
  pending proposal，不进入 Governance Ledger 或聊天页 Approval Center，也不拆成
  submit/approve/apply。
- 提交失败必须保留五项输入和当前步骤，允许用户就地修正或重试。
- 初始化应在第一个正式 Agent 会话/Frozen Core capture 之前完成，使第一次正式对话直接
  使用新人格；不得先创建空人格会话再热替换。
- 现有 Prompt 注入式 `First-Run Onboarding`、`ask_user ->
  laputa_propose_section_write` 初始化链路和 GUI `WELCOME_STORAGE_KEY` 判定均列入删除范围。
  技术配置向导若继续存在，也必须与 Laputa 人格初始化分离。

### P12：完整历史永久保留 Agent 的人格变化轨迹

- 四份 Persona 文档和 WORLD 的每次真实、成功内容变化都追加一个不可变历史版本，包括
  首次初始化、用户直接保存、接受 Agent 变更和从历史版本恢复后再次保存。
- 历史至少记录 revision、完整 Markdown 快照、相对上一版本的文本 Diff、actor/source、
  时间、变更原因和 base revision，使用户能够阅读 Agent 人格演变的连续轨迹。
- 历史版本不得覆盖、删除、重编号或原位编辑；默认不设自动裁剪、保留天数或数量上限。
  当前权威可以继续演进，但每次演进都只能追加新版本。
- 载入历史只产生本地草稿；再次保存产生新的头部版本，不回拨或抹除中间历程。
- 内容未变化的 no-op 保存不创建重复 revision；被拒绝的变更请求不属于人格变化，只保留
  请求决策审计，不写入文档历史。
- 完整历史属于审计/浏览面，不整体注入模型上下文。Frozen Core 和 WORLD 投影仍只读取
  当前权威并遵守既有会话冻结、scope 与预算边界。

## “不再有 JSON”的精确定义

禁止 JSON 的范围是**人格正文及其用户可见/领域内表达**：

- 权威正文；
- 编辑器内容；
- Proposal/变更正文；
- Diff 输入；
- Frozen Core 文本快照；
- Prompt 投影；
- 人格历史版本正文。

HTTP/Tauri 外层仍可使用 JSON/serde 传输结构化信封，例如 section 名、revision、时间和
错误码，但正文必须是 Markdown 字符串。Changelog/audit 元数据也可以结构化编码；这不
代表人格内容是 JSON。GUI 永远不得要求用户查看或编辑这些信封。

## 文件命名待实施前最终确认

推荐一个对象对应一个明确文件，例如：

- `IDENTITY.md`
- `RELATIONSHIP.md`
- `COMMITMENT.md`
- `PREFERENCES.md`

最终物理目录可位于 `.laputa/persona/` 或新的单一 Persona authority 目录。不得复用
旧 `.laputa/sections/` JSON 目录，也不得让多个旧文件合并映射到同一对象。文件名与
目录在实施计划中一次拍板，之后作为 clean-break deletion proof 的固定契约。

## 必须改写的架构面

实现不得继续把 Persona 塞入 `EvolutionProposal`。最小专用模型应等价表达：

```text
PersonaDocument(section, markdown, revision, updated_at)
PersonaChangeRequest(id, section, base_revision, before_markdown,
                     after_markdown, summary, state)
PersonaRevision(revision, markdown, diff, actor, created_at)
```

这些是人格领域对象；HTTP/Tauri 可以用结构化信封传输，但 Markdown 正文不能退化成
JSON object 或 patch。文本 Diff 展示组件未来可以被其他文档型功能复用，业务状态机
不得因此重新合并为通用 Proposal。

### Laputa / Core

- 独立 Persona 文档路径和原子文本写入；
- `LaputaSection` 或替代 Persona DTO 的正文类型；
- 用户直接保存、revision/CAS、changelog、真实 aligned text diff、历史和 rollback；
- PersonaChangeRequest 的创建、stale 检测与原子接受/拒绝；
- Frozen Core capture、section version、预算与 prompt render；
- 删除人格 JSON proposal type/route/parser 及 persona legacy migration；
- 保持 Persona 与普通 BML Memory 的类型和物理边界。
- 五份权威 absence-only 初始化检测、无预种子布局、原子创建与 incomplete recovery 状态；
- Persona/WORLD 永久 append-only revision store，历史与当前权威物理分离。

### Manager / Tauri

- Persona workspace/read/save/history/rollback/change-request 的窄接口；
- 正文使用 Markdown string，revision 作为显式并发前置条件；
- 删除 Persona 对通用 Evolution Proposal、governance projection 和 apply receipt 的依赖；
- 接受变更必须由单一服务端命令完成 CAS、authority write、history append 和状态终结；
- 传输层错误必须保留稳定 reason code，保存冲突不得覆盖用户草稿。
- first-run status/initialize 的窄接口；服务端返回 `uninitialized | ready | incomplete`，
  GUI 不自行推断或持久化第二套完成标记。

### GUI

- `PersonaMemoryView` 收敛为左侧四分区导航和单一中央工作区；
- `SectionEditor` 替换为当前文档、待审变更、历史三个显式状态；
- 删除 JSON format/parse/error、通用 proposal pending note 和永久生命周期审批栏；
- 当前文档提供 Markdown 源码、可折叠预览和直接保存；待审状态提供只读前后对比；
- 历史详情展示不可变 Markdown 版本与文本 Diff，并支持载入为未保存草稿；
- 保存失败和 revision 冲突保留草稿、选中分区与滚动位置。
- 首次引导一次收集 Identity/Relationship/Commitment/Preferences/WORLD，使用一个统一提交
  边界；incomplete 使用修复状态而非重弹初始化。

## 测试与删除证明要求

- 四个人格 Markdown 文档的创建、读取、修改、空态、并发冲突、历史和回滚；
- Markdown Diff 对新增/删除/修改、Unicode、换行和空文档的确定性测试；
- Frozen Core 同一会话冻结、下一会话生效、顺序与预算测试；
- Markdown 预览禁用 HTML/危险链接的 GUI 测试；
- 当前文档/待审变更/历史三态及窄屏切换，源码/预览折叠、草稿失败恢复和快捷键测试；
- 待审变更 before/after 对齐、接受/拒绝原子性、stale CAS 拦截和无 approve/apply 分裂；
- 历史载入只替换本地草稿、未保存草稿确认、保存后新增版本且旧历史不变；
- 五份权威全缺失/全存在/部分存在/空文件/损坏矩阵，初始化一次成功后不再出现；
- 五份权威与首批历史原子创建、任一点失败不留半初始化、重试保留输入；
- 初始化直写不产生 Proposal/Approval/Governance，且首个正式会话捕获完整 Frozen Core；
- Persona/WORLD 每次真实变化永久追加 revision，no-op/拒绝不生成文档版本，完整历史不进
  Prompt；
- Manager HTTP、Tauri 与真实桌面纵向 smoke；
- 符号和路径扫描证明人格 `.json`、JSON parse/format、旧 persona migration、
  Governance UI 和 fallback 均未回潮。

## 当前不决定

- Agent/系统在什么时机提出人格变更，以及变更摘要如何生成；
- 同一人格文档允许同时存在多少个待审请求及其排队策略；
- 最终 Persona authority 目录名；
- CodeMirror 6 的具体扩展集合与主题细节；
- Markdown 模板是否提供默认章节。

这些问题不得被实现者自行扩展为兼容层、自动合并器或第二套人格系统。

## 对先前记录的修订

本版本取代同一记录早先 P2 中“Persona 页面不再承担批准/拒绝”以及“Agent 自动建议
修改入口待决策”的宽泛表述。准确边界现为：Persona 删除安全审批和通用 Governance，
但在中央文档工作区保留专用的 Markdown 内容变更审查；聊天页 Approval Center 仍是
危险执行授权的唯一入口。v0.0.1 iteration log 保留为决策演进证据，不回写历史记录。

## 被取代的依据

以下材料保留为历史证据，但与本决策冲突的部分不再作为实施依据：

- `docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md`
- `docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md`
- `docs/logs/2026-08-laputa-persona-workspace/`
- `agent-diva-laputa/src/persona_retire.rs` 所代表的旧文件退休/迁移模型

早期文档中的 Markdown 编辑器原则可以复用；14-section、Memory 混入人格、隐式
Proposal apply、无 Diff 和旧文件兼容不能复用。
