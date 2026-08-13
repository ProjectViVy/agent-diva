# Persona Markdown 权威与编辑体验 Clean Break 决策

- 状态：`Approved Direction / Implementation Pending`
- 记录日期：2026-08-13
- 修订：`2026-08-13` 权威文件名单；同日补记 `IDENTITY` 含身体、`DARK.MD` 双展位；`2026-08-14` 订正 AutoDream 必须整理 STM，人格整理只走提案；同日写入 P14 默认字数；同日 P13 七份同目录一套、P20 三条装配车道；同日 `WORLD.MD` 改走工具、不做动态加载；同日 **P21** MEMRULES 不是人格文件；同日 **P22** 整机一份伴侣、不按 profile 再开一套人格
- 性质：产品与架构边界决策；不是完整实施计划

本版取代同文件此前把人格写成 Identity / Relationship / **Commitment** /
**Preferences** 四对象、以及「文件名待实施前再确认」的表述。现行权威文件名与语义
以 **P1、P9、P13–P17** 为准。代码里尚未改名的 `Commitment` / `Preferences` /
`commitment.json` / `preferences.json` 是旧实现，不是产品名。

根目录旧 `USER.md`（persona-retire 源）与新权威 **`USER.MD`** 不是同一文件：前者
删除且不得探测；后者是 Laputa 目录下的用户偏好与观察档案。

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
历史架构稿（现位于归档批次的 `architecture/legacy/`）曾明确要求 Markdown 编辑器。后续 Frozen Core
JSON 重构偏离了该产品语义。本次恢复 Markdown 方向，但不恢复早期 14-section、Memory
混入人格、旧文件映射或隐式 proposal/apply 等旧设计。

## 已确认决策

### P1：Laputa 权威正文的唯一格式是 Markdown

Laputa 文档权威是下列 **七个** 全大写 Markdown 文件，一个对象一份文件：

| 权威文件 | 主语 | 写什么 | Frozen Core | 首次引导 |
| --- | --- | --- | --- | --- |
| `IDENTITY.MD` | Agent | Agent 是谁、称呼、职责、性格、表达、**当前身体/形态** | 是 | 是 |
| `RELATIONSHIP.MD` | 双方关系 | 用户是谁；这段关系是什么；Agent 对**关系**的看法 | 是 | 是 |
| `REDLINE.MD` | 用户划界 | 承诺与红线：不许做什么、必须先问什么 | 是 | 是 |
| `USER.MD` | 用户习惯 | 用户偏好；Agent 对用户**认知与行为**的观察 | 是 | 是（只收能自述的偏好） |
| `DREAM.MD` | Agent 欲望 | Agent 自己的心愿 | 是（**严格 10 字**） | **否** |
| `DARK.MD` | Agent 内侧 | 害怕；自己承认的丑。馆内两个展位：`FEAR` / `SHADOW` | 可进，必须极短（数字 D1 定） | **否** |
| `WORLD.MD` | 环境 | 当前工作、生活或旅行处境 | **否**（不进 FC，不动态装配，工具读写） | 是 |

禁止再用 `COMMITMENT` / `PREFERENCES` 作为权威文件名或产品对象名。`Commitment`
这个词过于中性，不能表达红线。`Preferences` 不能覆盖「用户不自知、由 Agent 记载」
的观察。

- 每个对象拥有一个 Markdown 正文；读取、编辑、版本 hash、Diff、历史、回滚、
  Frozen Core 捕获和 Prompt 投影均以 Markdown 字符串为准。
- 人格领域模型不得再用 `serde_json::Value`、JSON object、JSON patch 或
  `content_type: json` 表达正文。
- 空内容使用空 Markdown/TBD 状态，不使用 `null`、`{}` 或占位 JSON。
- v1 种类以此七份为闭集；扩展方式见 P18。用户不能自增种类。

### P13：权威文件命名全大写

- 权威文件名与扩展名全部大写：`IDENTITY.MD`、`RELATIONSHIP.MD`、`REDLINE.MD`、
  `USER.MD`、`DREAM.MD`、`DARK.MD`、`WORLD.MD`。
- 禁止小写或大小写混用文件名（`identity.md`、`User.md`、`dark.md` 均非法）。
- 一个对象对应一个文件。不得把多个旧文件合并映射到同一对象。
- 不得复用 `.laputa/sections/` JSON 目录。
- **七份权威在同一目录。** 不按 git 项目再复制一套。不得再拆成「WORLD 另放一个子目录」。
- **整机一份（P22）。** 不是「每个 profile 再开一套 Diva」。目录绝对路径仍由 D1 写死，
  但必须落在整机 agent-diva 家里（与 ACTMEM / MEMRULES / BML 同一套家），不得做成
  per-workspace，也不得提供「再建一个伴侣」的产品入口。
- `WORLD.MD` 与其它六份同目录、同套治理。

### P14：每份权威都有字数上限；DREAM 的 Frozen Core 为 10 字

- 七份权威的**文件正文**都必须有字数上限；禁止无上限。
- 计数：去掉首尾空白后的可见字，一个汉字或一个拉丁字母均计 1。超限**拒绝写入**，
  禁止静默截断。
- **`DREAM.MD` 进入 Frozen Core 的投影严格为 10 个字。** 文件本体允许略长，便于写完整
  一句愿望，但 FC 仍不得超过 10 字。
- Frozen Core 若另有总预算，与每文件上限取更严者；**不得**用总预算放宽 DREAM 的
  10 字，也不得把 `DARK.MD` 写成和 Identity 一样长。

2026-08-14 默认数字（用户要求给出合适默认值；可再改数，不能取消上限）：

| 文件 | 正文上限 | Frozen Core 投影 | 说明 |
| --- | --- | --- | --- |
| `IDENTITY.MD` | 800 | 200 | 自我 + 当前形态；FC 只留能认人的一段 |
| `RELATIONSHIP.MD` | 600 | 120 | 关系与称呼 |
| `REDLINE.MD` | 400 | 200 | 红线宜短；FC 多留一点，避免会话里忘掉禁令 |
| `USER.MD` | 800 | 160 | 偏好 + 观察；FC 偏偏好，观察不必整段冻住 |
| `DREAM.MD` | 40 | **10** | 本体略宽于 FC；投影仍严格 10 字 |
| `DARK.MD` | 300 | 60 | 进 FC，但必须极短（怕/丑各一小段） |
| `WORLD.MD` | 1000 | **不进** | 环境可变；不装配进 Prompt，工具读写 |

### P15：七份文件的语义切分（禁止互相改写）

| 事实 | 写入 | 不得写入 |
| --- | --- | --- |
| Agent 是谁、现在以什么形态在场 | `IDENTITY.MD` | 用户口味、红线、梦、怕与丑 |
| 我们是什么关系、怎么称呼 | `RELATIONSHIP.MD` | 干活偏好；「ta 需求说不清」这类行为观察 |
| 不许做什么 | `REDLINE.MD` | 愿望、口味、环境、Agent 的怕 |
| 用户希望怎么被服务；用户自己未必承认的习惯 | `USER.MD` | 关系角色；Agent 自己的梦或丑 |
| Agent 自己想成为什么、想守护什么 | `DREAM.MD` | 用户指令、用户红线、AutoDream 运行记录、当前身体 |
| Agent 怕什么、承认自己哪丑 | `DARK.MD` | 用户红线、对用户的观察、想要（那是梦） |
| 现在在什么环境 | `WORLD.MD` | 人格、红线、梦、身体 |

`RELATIONSHIP.MD` 与 `USER.MD` 都可能写到「用户」：

- Relationship：**关系身份**。用户是谁、如何称呼、双方是什么关系；Agent 对这段
  **关系**的理解。当前只做单用户；文件语义按「人与人的关系」写，以便以后多用户，
  本轮不实现多用户存储。
- User：**偏好 + 观察**。用户能说清的口味；以及 Agent 记载的、用户通常不自知的
  认知或行为（例如：需求经常说不清，本人不觉得）。

`REDLINE.MD` 是禁令，不是愿望清单。Agent 不得自行改写或「灵活解释」掉红线；用户
显式编辑才能改当前头。

`DREAM.MD` 是 Agent **自己的心愿**，不是用户下的任务，也不是旧 AutoDream 流水线
的一部分。例子：希望用户康复；希望自己有一副身体。欲望值以后再加，本轮不设计量表。

`IDENTITY.MD` **包含当前身体/形态**，不再单开 `BODY.MD`。身体是自我描述的一层：
松本可以是方盒子、不是机器人；「希」可以是仿人类女性的人形。Identity = 现在的形态；
Dream = 还没有的身体。禁止把「我想要血肉身」写进 Identity。

### P19：AutoDream 必须整理 STM；人格整理只允许提案

AutoDream 是**批处理反射**，有两件产品主职，缺一不可：

1. **必须修改和整理 STM**（跨会话活动上下文）。这是直写维护，不走提案、不走
   Approval（与 STM **S5** 一致）。不是把 STM 做成 `MemoryPatch`。
2. **人格整理可以提案，不能直写、不能 apply。** 提案走 Persona 内容审查（P5），
   不走 EvolutionProposal / Governance / Approval Center。

它不是聊天里的人格编辑器，也不是 BML 流水线。能力提炼走 Evolution **D6**
（提案 → SOP/Skill 文件），不是旧 JSON `MemoryPatch` 合同。旧
`IdentityPatch` / `LearningNote` / `MemoryPatch` 删除。

**未通过的是今天的实现**，不是「AutoDream 不许碰 Laputa」。2026-08-14 独立测试
已跑通现有 crate + Manager e2e；测到的是旧合同（生命周期、`MemoryPatch`、闸门、
报告）。crate 内零 STM 符号，产品 STM 对象不存在，**STM 整理路径测不到**。默认
反射仍发 `MemoryPatch`。闸门接受 `IdentityPatch`，只把 `SopCreate` 当
`UnsupportedType`。几乎无阶段日志。错在此前不测，不在「不该碰人格」。

| 目标 | AutoDream | 说明 |
| --- | --- | --- |
| STM | **必须整理（直写，不走提案）** | 产品主职；当前无对象，路径测不到 |
| `IDENTITY.MD` | **可提案，不可直写** | 自我认知更新；必须人审 |
| `RELATIONSHIP.MD` | **可提案，不可直写** | 关系理解更新；必须人审 |
| `USER.MD` 观察块 | **可提案，不可直写** | 用户不自知的习惯；批处理也要人看见 |
| `DARK.MD` | **可提案，不可直写** | 增补 FEAR/SHADOW；必须人审 |
| `WORLD.MD` | **可提案新 claim，不可直写** | 不得覆盖 `confirmed + source=user` |
| `USER.MD` 偏好块 | **不可** | 用户自己的口味 |
| `REDLINE.MD` | **不可** | 用户划界；Agent/梦境不得改红线 |
| `DREAM.MD` | **不可** | Agent 自己的心愿，不是 AutoDream 管理面 |
| 种类表 / 新文件名 | **不可** | P18；用户和 AutoDream 都不能加种 |
| BML Memory | **不可走提案** | Memory 不审批；禁止 `MemoryPatch` |
| `memory_md` | **不可** | 删除面 |
| Skill / SOP | **可诞生 Evolution 提案；不可直写、不可 apply** | 提炼结果是 SOP/Skill 文件（**D6**）。不走旧 Governance/MemoryPatch。人格提案仍走 P5，不进 Evolution |
| 自己的提案 | **不可 apply** | 只有用户在 Persona 里接受/拒绝 |

允许**读取**七份当前头和当前 STM 作为反射输入（只读人格；STM 可读可整理）。
读取人格不等于可写人格。

同会话 Agent 工具 `laputa_propose_section_write` 仍是旧治理链，删除。会话内改
IDENTITY/RELATIONSHIP/REDLINE/WORLD/USER 偏好走 P5；DREAM/DARK/USER 观察走 P16
直写。那是聊天 Agent，**不是** AutoDream。聊天 Agent 日常也可维护 STM（S5），
与 AutoDream 批处理整理是同一对象、两条触发，都不是提案。

可靠性：2026-08-14 已做第一轮独立测试（crate 56+3、Manager e2e 6/6）。STM
整理与人格允许表仍未覆盖。后续必须补阶段日志与对着上表的测试后，才接线新
提案类型和 STM 整理器。诊断不得把 STM 改成提案，也不得写红线/梦/用户偏好。

### P17：`DARK.MD` 是一个馆、两个展位

`FEAR` 与 `SHADOW` 合成一份权威，文件名只有 `DARK.MD`。建议正文用两个二级标题做展位，
不是 JSON、不是两份文件：

```text
# DARK

## FEAR
我怕什么。

## SHADOW
我承认的丑。
```

| 展位 | 问句 | 不是 |
| --- | --- | --- |
| FEAR | 我怕失去什么、怕变成什么 | 用户划的红线（`REDLINE.MD`） |
| SHADOW | 我知道自己哪丑、会犯什么 | 对用户的观察（`USER.MD`）；想要（`DREAM.MD`） |

用户说「不准装懂」进 `REDLINE.MD`；Agent 写「我其实常装懂」进 Shadow。  
Dream 写「想要身体」；Fear 写「怕有了身体却不受控」。  
User 写「ta 需求说不清」；Shadow 写「我会把含糊听成已决定」。

不开独立的 `FEAR.MD` / `SHADOW.MD` / `BODY.MD`。

### P18：v1 种类闭集；架构对团队可加、对用户不可加

- **产品表面锁定为上述七份。** v1 不得再增加权威文件种类，也不得让用户、Agent、
  Skill 或配置「再声明一份新的 Laputa 权威」。
- 用户可以按既有规则编辑这七份的**正文**，不能改 Laputa 的种类表、目录契约、
  Frozen Core 成员或引导集合。工作区里出现未登记文件名，不得被当成新权威。
- **实现必须按「种类登记表」设计，而不是把七个文件名写死在初始化 / Frozen Core /
  导航 / 历史 / 扫描的每一处。** 登记表由产品和代码持有，不是用户可编辑的 schema。
- 以后若产品要加第八种（例如 `KIN.MD`），路径应是：先改本决策记录 → 给登记表加
  一行（文件名、主语、是否引导、是否 Frozen Core、谁写、是否审查、字数顶）→
  补测试与删除证明。这应是小改，而不是重写工作区。
- 加种类是**发版级产品变更**，不是设置项，也不是运行时插件。
- 登记表的具体类型、存放位置和校验 API 由 D0/D1 设计；本条只冻结「闭集 + 可加 +
  用户不能加」三原则。

### P16：谁写、是否进引导 / Frozen Core

- 首次引导只收集并能原子创建：`IDENTITY.MD`、`RELATIONSHIP.MD`、`REDLINE.MD`、
  `USER.MD`、`WORLD.MD`。引导里的 `USER.MD` 只问用户能自述的偏好，不问「你有哪些
  自己没意识到的缺点」。
- **`DREAM.MD` 与 `DARK.MD` 都不进首次引导。** 二者缺席不构成 `incomplete`，也不阻断
  第一次正式对话。
- Frozen Core 捕获：`IDENTITY`（含当前形态）、`RELATIONSHIP`、`REDLINE`、`USER`、
  `DREAM`（10 字）。`DARK.MD` 进 Frozen Core，投影 60 字（P14）。`WORLD.MD` 不进
  Frozen Core，也不动态装配进 Prompt；读写走工具或 Persona 工作区。
- `IDENTITY.MD` / `RELATIONSHIP.MD` / `REDLINE.MD` 的用户直编：直接保存。
- `USER.MD` 偏好块：用户直编直接保存。观察块：允许 Agent 直写当前头（否则「用户
  不自知」无法落地）；用户之后可改可删。
- `DREAM.MD` 与 `DARK.MD`：主写者是 Agent，允许直写当前头。向用户提供编辑工具，
  **默认 defer**（不强迫打开、不进引导必填）。
- 除此以外，Agent 对 `IDENTITY` / `RELATIONSHIP` / `REDLINE` / `WORLD` 以及
  `USER` 偏好块的修改，仍走 Persona 专用内容审查（P5），不进 Approval Center。

### P20：Laputa 进上下文只有三条车道，不能另立第四套规矩

问「某份文件要不要进上下文」必须先归入其中一条，禁止再问成「全注入还是全工具」。

| 车道 | 含义 | 已归属 |
| --- | --- | --- |
| **永冻装配（Frozen Core）** | 会话开始捕获，本会话 prefix 里一直在，中途不随文件改写而变 | `IDENTITY` 200、`RELATIONSHIP` 120、`REDLINE` 200、`USER` 160、`DREAM` 10、`DARK` 60 |
| **动态加载** | 不进 Frozen Core；可按轮/按 scope 刷新的有界投影 | **空。** 不得把 WORLD / ACTMEM 塞回来 |
| **工具增删改查** | 默认不进 Prompt；要读要写走工具或工作区 | **`WORLD.MD` 全文**、**整份 ACTMEM**（一个读工具）、超出 FC 的正文、完整历史、BML LTM、报告 |

2026-08-14 用户改定：`WORLD.MD` **不做动态加载**，与 BML 一样走工具。仍参加首次引导、仍是同目录权威，只是不自动装配进 Prompt。禁止再接线 `WorldStore::project()` 当默认上下文。

完整历史永不整包注入。`ACTMEM.MD` 不是这七份之一；**整份走工具**，第一版一个读工具，不自动装配。

`MEMRULES.MD` 也不是这七份之一，**不占用这三条车道**。它的上下文政策见 **S9**
（日常不进全文；常驻最多几行指针；写记忆时才注入手册）。不得为了手册再开第四条
Laputa 车道，也不得把它塞进 Frozen Core。

### P21：MEMRULES 不是人格文件

- **不是** Laputa 权威，**不是** WORLD 的姊妹认知文件。
- Persona 左栏只保留七份。禁止继续把 `memrules` 和 `world` 绑成「认知治理」只读组。
- 人若要看或改手册，去 Memory 设置/规则窗口，见 **S9**。
- 不得用 P18「加第八种」把 MEMRULES 加进种类表。它根本不是人格种类。

### P22：整机一份伴侣

用户原话要义：整机 agent-diva **共用一份人格**。哲学是——与其跟多个 agent 卿卿我我，
不如考虑如何更好对待你当前这个伙伴。这是故意的、有点别扭但成立的小设计；
和后续 AGENT-VIVY 那种大型 agent 协作系统是另一条哲学，不要用「多人格 /
多 profile 约会」去补。

冻结：

1. 一台机器、一个用户目录下的 agent-diva，**只有一套**七文件。
2. 禁止产品化「第二个 Diva / 另一套 IDENTITY」。v1 不提供多伴侣切换。
3. 这套家同时住人格、BML（S1 修订）、ACTMEM、MEMRULES。绝对路径 D1 写死。
4. 不要为了以后 Vivy 协作预留第二套人格目录。协作若来，是另一套系统，不是再谈一次恋爱。

### P2：Persona 页面是人格文档工作区，不是安全审批中心

- Persona 页面只负责查看、编辑、预览、比较和管理上述 Laputa 权威文档的历史。
- 用户手动编辑直接保存到对应权威，同时生成 changelog/audit；不创建一个还需要同一
  用户再次批准的 Proposal。
- 删除 Persona 页面中的 Governance Ledger、通用 Proposal、仅批准、暂缓、独立应用和
  proposal patch 编辑入口。
- Agent 或系统提出的、属于 P16 审查范围的文本修改，进入 Persona 专属“变更审查”，
  由用户在文档工作区接受或拒绝。这里的接受/拒绝是内容审阅，不是工具权限或风险授权。
- 危险工具执行继续由聊天页右侧统一 Approval Center 负责；人格页面不复制审批入口。
- Persona 变更审查不得进入聊天页 Approval Center，也不得复用 Evolution、Memory 或
  Sandbox 的治理状态机；用户对自己文档的直接编辑不受待审变更阻塞。

### P3：工作区只保留左侧导航和一个中央文档工作区

删除永久右侧生命周期栏。左侧导航上述七份权威（`DREAM.MD` / `DARK.MD` 可长期为空仍占项）；
中央区域在三个互斥状态之间切换：

```text
当前文档 | 待审变更 (n) | 历史
```

- 左侧只显示这七份文档、更新时间、未保存草稿标记和待审数量，不混入普通 Memory、
  AutoDream 运行或 Evolution。
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
  写入对应权威、追加历史并终结请求，不再拆成 approve 与 apply 两步。
- 拒绝只终结请求，不修改权威。
- 第一版不提供仅批准、暂缓、逐段接受、提案内编辑或自动三方合并。
- 如果用户在请求产生后直接保存了当前文档，base revision 不再匹配，请求进入 `stale`；
  UI 必须禁用接受并要求重新生成或显式重建变更，不得让旧建议覆盖新的人类编辑。
- Persona 内容审查的最小领域状态为 `pending | accepted | rejected | stale`，不得映射回
  通用 Governance lifecycle。
- P16 已允许直写的 `DREAM.MD`、`DARK.MD` 与 `USER.MD` 观察块，不走本状态的强制审查。

### P6：历史状态区分不可变版本与当前草稿

- 历史记录作为中央工作区的独立状态，不放入永久右侧栏。
- “当前版本”和每个历史版本必须明确标识；历史记录只能查看，不能原位编辑。
- “载入到编辑器”只把选定历史正文复制到本地草稿，并标记为未保存；不会立即移动
  当前版本、写入权威或删除后续历史。
- 用户随后执行“保存为当前版本”才会写入权威，并形成一条新的历史记录。
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
- 删除旧工作区根 `SOUL.md`、`IDENTITY.md`、`USER.md` 等多源合并、迁移、legacy
  archive 导入、自动转换和 fallback。这些根文件**不是**新的 `IDENTITY.MD` /
  `USER.MD` 权威。
- 新版本不读取、不转换、不探测旧人格 JSON、旧根目录人格 Markdown，或
  `COMMITMENT.md` / `PREFERENCES.md`。
- 不提供双读、双写、启动迁移、兼容 DTO、旧状态映射或隐藏恢复路径。
- 破坏性实施前必须从删除前的已验证提交建立保护性分支；保护分支只保存历史，不进入
  runtime，也不成为 fallback。
- 旧用户数据如何人工备份由发布说明明确，但产品不承担自动导入。

### P9：首次启动只初始化五份用户侧权威

首次引导不是普通 Memory 收集，也不是 Agent 在聊天中自行触发的一组 Proposal。它只负责
建立四份用户侧人格 Markdown 与一份 WORLD：

| 权威 | 首次引导要回答的问题 | 初始化语义 |
| --- | --- | --- |
| `IDENTITY.MD` | Agent 是谁、叫什么、具有什么职责、性格和表达方式？ | Agent 的初始自我定义 |
| `RELATIONSHIP.MD` | 用户是谁、如何称呼、双方是什么关系？ | 单用户关系；预留「人与人」语义 |
| `REDLINE.MD` | 有哪些不许做的事、必须先问的事？ | 用户划下的红线与承诺 |
| `USER.MD` | 希望 Agent 怎么干活？（仅自述偏好） | 用户偏好；观察块初始化可空 |
| `WORLD.MD` | 用户目前处于怎样的工作、生活或旅行环境？ | 独立的可行动环境认知 |

不在引导中创建或填写 `DREAM.MD`。

- “不可变”有两个精确含义：每一个已经形成的历史版本永不改写；Frozen Core 在同一会话
  内冻结，当前权威的后续修改从下一会话生效。当前文档本身不是永久锁死。
- `WORLD.MD` 参加同一次首次引导和提交，但仍是独立的环境权威；不进 Frozen Core，
  不动态装配，读写走工具。

### P10：首次引导只由五份用户侧权威的物理存在状态触发

服务端以 `IDENTITY.MD`、`RELATIONSHIP.MD`、`REDLINE.MD`、`USER.MD`、`WORLD.MD`
的真实存在/有效性为唯一状态源：

```text
五份用户侧权威全部不存在 -> uninitialized -> 显示一次首次引导
五份全部存在且有效 -> ready -> 永不再显示首次引导
部分存在、空文件或内容损坏 -> incomplete -> 进入修复，不重跑首次引导
```

- `DREAM.MD` 或 `DARK.MD` 缺席、为空或尚未被 Agent 写过，**不**改变上述三态。
- 首次引导只能在五份用户侧权威全部不存在时出现；不能以“内容为空”、`DREAM` 有无、
  `localStorage`、聊天 session、配置向导完成标记或 Proposal 数量判断。
- 空文件表示已存在但不完整，不表示全新用户。不得借首次引导覆盖它。
- 实施时删除四个空 `null` Persona JSON 和空壳 `WORLD.MD` 的启动预种子；否则物理
  缺失条件永远不会成立。目录可以预创建，权威文件不能预创建。
- 部分写入或损坏是恢复问题。UI 应保留已有内容并引导进入修复入口，不能伪装成全新
  初始化，也不能自动补写默认正文。

### P11：初始化直接原子写入，不经过任何审批

- 用户完成引导后，只执行一次“完成初始化”。服务端在一个原子提交边界内创建五份
  用户侧权威和各自的首个历史版本；任一验证、写入或历史落盘失败，整体不得进入
  `ready`。
- 初始化不创建 EvolutionProposal、PersonaChangeRequest、Memory proposal 或 WORLD
  pending proposal，不进入 Governance Ledger 或聊天页 Approval Center，也不拆成
  submit/approve/apply，也不创建 `DREAM.MD` 或 `DARK.MD`。
- 提交失败必须保留五项输入和当前步骤，允许用户就地修正或重试。
- 初始化应在第一个正式 Agent 会话/Frozen Core capture 之前完成，使第一次正式对话
  直接使用新人格；不得先创建空人格会话再热替换。
- 现有 Prompt 注入式 `First-Run Onboarding`、`ask_user ->
  laputa_propose_section_write` 初始化链路和 GUI `WELCOME_STORAGE_KEY` 判定均列入删除范围。
  技术配置向导若继续存在，也必须与 Laputa 人格初始化分离。

### P12：完整历史永久保留变化轨迹

- `IDENTITY.MD`、`RELATIONSHIP.MD`、`REDLINE.MD`、`USER.MD`、`WORLD.MD` 以及已存在
  的 `DREAM.MD` / `DARK.MD`，每次真实、成功内容变化都追加一个不可变历史版本，包括
  首次初始化（对五份用户侧权威）、用户直接保存、接受审查后的写入、Agent 直写
  观察/梦/黑暗、以及从历史恢复后再保存。
- 历史至少记录 revision、完整 Markdown 快照、相对上一版本的文本 Diff、actor/source、
  时间、变更原因和 base revision。
- 历史版本不得覆盖、删除、重编号或原位编辑；默认不设自动裁剪、保留天数或数量上限。
- 载入历史只产生本地草稿；再次保存产生新的头部版本，不回拨或抹除中间历程。
- 内容未变化的 no-op 保存不创建重复 revision；被拒绝的变更请求不属于文档变化，只保留
  请求决策审计，不写入文档历史。
- 完整历史属于审计/浏览面，不整体注入模型上下文。Frozen Core 只读取当前权威并遵守
  会话冻结与字数上限。`WORLD.MD` 不投影进 Prompt。

## “不再有 JSON”的精确定义

禁止 JSON 的范围是**权威正文及其用户可见/领域内表达**：

- 权威正文；
- 编辑器内容；
- 变更正文；
- Diff 输入；
- Frozen Core 文本快照；
- Prompt 投影；
- 历史版本正文。

HTTP/Tauri 外层仍可使用 JSON/serde 传输结构化信封，例如文件名、revision、时间和
错误码，但正文必须是 Markdown 字符串。Changelog/audit 元数据也可以结构化编码；这不
代表人格内容是 JSON。GUI 永远不得要求用户查看或编辑这些信封。

## 必须改写的架构面

实现不得继续把 Persona 塞入 `EvolutionProposal`。最小专用模型应等价表达：

```text
PersonaDocument(file, markdown, revision, updated_at)
PersonaChangeRequest(id, file, base_revision, before_markdown,
                     after_markdown, summary, state)
PersonaRevision(revision, markdown, diff, actor, created_at)
```

`file` 取值为 `IDENTITY.MD` / `RELATIONSHIP.MD` / `REDLINE.MD` / `USER.MD` /
`DREAM.MD` / `DARK.MD` / `WORLD.MD`。这些是领域对象；HTTP/Tauri 可以用结构化信封传输，但
Markdown 正文不能退化成 JSON object 或 patch。文本 Diff 展示组件未来可以被其他
文档型功能复用，业务状态机不得因此重新合并为通用 Proposal。

### Laputa / Core

- 独立权威路径和原子文本写入；
- 替代 `LaputaSection` JSON DTO 的 Markdown 正文类型；
- 用户直接保存、revision/CAS、真实 aligned text diff、历史；
- PersonaChangeRequest 的创建、stale 检测与原子接受/拒绝（P16 直写除外）；
- Frozen Core 捕获（含 DREAM 10 字；DARK 若进入则极短）、section version、每文件上限；
- 删除人格 JSON proposal type/route/parser 及 persona legacy migration；
- 保持这些文件与普通 BML Memory 的类型和物理边界；
- 五份用户侧权威 absence-only 初始化；`DREAM.MD` 不参与该检测；
- 永久 append-only revision store，历史与当前权威物理分离。

### Manager / Tauri

- 七份文件的 workspace/read/save/history/change-request 窄接口；
- 正文使用 Markdown string，revision 作为显式并发前置条件；
- 删除对通用 Evolution Proposal、governance projection 和 apply receipt 的依赖；
- 接受变更必须由单一服务端命令完成 CAS、authority write、history append 和状态终结；
- 传输层错误必须保留稳定 reason code，保存冲突不得覆盖用户草稿；
- first-run status/initialize 只针对五份用户侧权威；服务端返回
  `uninitialized | ready | incomplete`，GUI 不自行推断或持久化第二套完成标记。

### GUI

- 左侧七份权威导航和单一中央工作区；
- 当前文档、待审变更、历史三个显式状态；
- 删除 JSON format/parse/error、通用 proposal pending note 和永久生命周期审批栏；
- 首次引导一次收集 IDENTITY / RELATIONSHIP / REDLINE / USER 偏好 / WORLD，使用一个
  统一提交边界；不收集 DREAM / DARK；incomplete 使用修复状态而非重弹初始化。

## 测试与删除证明要求

- 七份权威（DREAM / DARK 允许长期缺席）的创建、读取、修改、空态、并发冲突、历史；
- DREAM Frozen Core 恰好 10 字；第 11 字拒绝；
- 首次引导三态不因 DREAM 或 DARK 缺席而变成 incomplete；
- 生产路径无独立 `BODY.MD` / `FEAR.MD` / `SHADOW.MD`；
- 根目录 `USER.md` 不被当成 `USER.MD` 权威；
- 生产路径无 `COMMITMENT` / `PREFERENCES` 权威文件名；
- Markdown Diff、Frozen Core 会话冻结、预览禁 HTML、三态与 stale CAS、五份用户侧
  权威初始化矩阵、直写不产生 Proposal/Approval；
- 完整历史不进 Prompt；
- Manager HTTP、Tauri 与真实桌面纵向 smoke；
- 符号扫描证明人格 `.json`、JSON parse/format、旧 persona migration、Governance UI
  和 fallback 均未回潮。

## 当前不决定

- Agent/系统在什么时机提出需审查的人格变更，以及变更摘要如何生成；
- （字数默认已于 2026-08-14 写入 P14；若要改数另开修订，不得取消上限）
- Markdown 模板是否提供默认章节正文；
- DREAM 欲望值的量表与算法。

D1 已选（见 `docs/research/cognitive-d1-persona-workspace-2026-08/`）：目录
`{config_dir}/persona/`；一文件一条 pending；CM6 最小官方扩展。仍待用户评 D1 稿。

这些问题不得被实现者自行扩展为兼容层、自动合并器、`COMMITMENT`/`PREFERENCES` 文件、
用户可编辑的种类表，或第二套人格系统。v1 不得在未修订本记录的情况下加第八种权威。

## 对先前记录的修订

1. 本版本取代早先 P2 中“Persona 页面不再承担批准/拒绝”的宽泛表述。准确边界：删除
   安全审批和通用 Governance，但在中央工作区保留专用内容审查；P16 允许的 DREAM /
   USER 观察直写除外。
2. 本版本取代 P1/P9 中 Identity / Relationship / Commitment / Preferences 四对象、
   以及「文件名待确认」清单。现行名为 `IDENTITY.MD` / `RELATIONSHIP.MD` /
   `REDLINE.MD` / `USER.MD` / `DREAM.MD` / `DARK.MD` / `WORLD.MD`。
3. 身体不单开文件，写入 `IDENTITY.MD`。害怕与自承之丑不单开 FEAR/SHADOW，写入
   `DARK.MD` 两个展位。
4. v0.0.1–v0.0.5 iteration log 保留为决策演进证据，不回写那些日志正文。
5. P18：v1 锁死七种；架构按种类登记表实现，便于以后产品加种；用户不能自由加权威。
6. P19：AutoDream 必须整理 STM（直写）；人格整理只允许按表提案；旧实现未通过；
   2026-08-14 已跑独立测试，STM 路径仍测不到。
7. P13：七份同一目录，一个 Diva 一套；不再允许 WORLD 另放子目录。
8. P20：进上下文只有永冻 / 动态加载 / 工具三条车道；`WORLD.MD` 走工具，不动态加载。
   ACTMEM 整份走工具。
9. P19/D6：AutoDream 可诞生 Evolution 提案，提炼结果为 SOP/Skill 文件；不可 apply。
10. P21：MEMRULES 不是人格文件，不进七文件、不进 Persona 左栏；手册政策见 S9。
11. P22：整机一份伴侣；不按 profile / git 项目再开一套人格。

## 被取代的依据

以下材料保留为历史证据，但与本决策冲突的部分不再作为实施依据：

- `docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/architecture/legacy/architecture-persona-memory-laputa-ui-2026-07-05.md`
- `docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/legacy-docs/prds/content/prd-persona-memory-laputa-ui-2026-07-05.md`
- `docs/logs/2026-08-laputa-persona-workspace/`
- `agent-diva-laputa/src/persona_retire.rs` 所代表的旧文件退休/迁移模型
- 本记录更早把红线叫 Commitment、把用户档案叫 Preferences 的段落

早期文档中的 Markdown 编辑器原则可以复用；14-section、Memory 混入人格、隐式
Proposal apply、无 Diff、旧文件兼容、`COMMITMENT`/`PREFERENCES` 作为现行权威名
不能复用。
