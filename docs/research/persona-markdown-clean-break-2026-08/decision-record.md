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

### P2：Persona 页面是人格文档工作区，不是审批中心

- Persona 页面只负责查看、编辑、预览、比较和管理人格文档历史。
- 用户手动编辑直接保存到人格权威，同时生成 changelog/audit；不创建一个还需要同一
  用户再次批准的 Proposal。
- 删除 Persona 页面中的 Governance Ledger、批准、仅批准、拒绝、暂缓、应用和
  proposal patch 编辑入口。
- 危险工具执行继续由聊天页右侧统一 Approval Center 负责；人格页面不复制审批入口。
- Agent 自动建议人格修改是否需要审批、采用何种入口，留作独立决策，不能阻塞用户
  对自己人格文档的直接编辑。

### P3：编辑体验使用“源码 + 人类预览 + Diff”

宽屏主结构：

```text
人格分区导航 | Markdown 源码编辑器 | 人类可读预览 / Diff
```

- 左侧：四个人格分区及其状态、更新时间；不混入普通 Memory、AutoDream 或 Evolution。
- 中间：极简 VS Code 风格 Markdown 编辑器，至少支持行号、Markdown 语法高亮、当前行、
  查找、撤销/重做、滚动、`Ctrl/Cmd+S` 和明确 dirty 状态。
- 右侧：安全 Markdown 渲染预览；顶部可在 `预览` 与 `Diff` 间切换。
- Diff 比较当前权威版本与尚未保存的草稿，按文本行展示新增、删除与修改；不得比较
  JSON 序列化结果。
- 窄屏退化为 `编辑 / 预览 / Diff` 单工作区标签，不把三个区域挤在同一行。
- 页面主操作是保存人格文档；历史和回滚属于次级文档管理动作。

### P4：采用成熟编辑器能力，不手写 textarea 伪装

- 实施前评估 CodeMirror 6 与当前 Vue/Vite/Tauri 依赖和包体；默认优先 CodeMirror 6，
  完整 Monaco 只有在确需语言服务时才考虑。
- 现有 `markdown-it` 可用于预览，必须保持原始 HTML 禁用，并核查链接协议和外部资源
  边界。
- 不通过给普通 textarea 增加行号背景或 CSS 高亮声称已实现代码编辑器。

### P5：破坏性 Clean Break，不做任何人格旧格式兼容

- 删除 `.laputa/sections/identity.json`、`relationship.json`、`commitment.json`、
  `preferences.json` 作为人格权威的能力。
- 删除旧 `SOUL.md`、`IDENTITY.md`、`USER.md` 等多源合并、迁移、legacy archive 导入、
  自动转换和 fallback。
- 新版本不读取、不转换、不探测旧人格 JSON 或旧人格 Markdown 文件。
- 不提供双读、双写、启动迁移、兼容 DTO、旧状态映射或隐藏恢复路径。
- 破坏性实施前必须从删除前的已验证提交建立保护性分支；保护分支只保存历史，不进入
  runtime，也不成为 fallback。
- 旧用户数据如何人工备份由发布说明明确，但产品不承担自动导入。

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

### Laputa / Core

- 独立 Persona 文档路径和原子文本写入；
- `LaputaSection` 或替代 Persona DTO 的正文类型；
- 用户直接保存、revision/CAS、changelog、unified text diff、历史和 rollback；
- Frozen Core capture、section version、预算与 prompt render；
- 删除人格 JSON proposal type/route/parser 及 persona legacy migration；
- 保持 Persona 与普通 BML Memory 的类型和物理边界。

### Manager / Tauri

- Persona workspace/read/save/history/rollback 的窄接口；
- 正文使用 Markdown string，revision 作为显式并发前置条件；
- 删除 Persona 对通用 Evolution Proposal、governance projection 和 apply receipt 的依赖；
- 传输层错误必须保留稳定 reason code，保存冲突不得覆盖用户草稿。

### GUI

- `PersonaMemoryView` 收敛为四分区人格工作区；
- `SectionEditor` 替换为 Markdown 源码编辑器、预览和 Diff；
- 删除 JSON format/parse/error、proposal pending note 和生命周期审批栏；
- 历史详情展示 Markdown 版本与文本 Diff，并支持复制/回滚；
- 保存失败和 revision 冲突保留草稿、选中分区与滚动位置。

## 测试与删除证明要求

- 四个人格 Markdown 文档的创建、读取、修改、空态、并发冲突、历史和回滚；
- Markdown Diff 对新增/删除/修改、Unicode、换行和空文档的确定性测试；
- Frozen Core 同一会话冻结、下一会话生效、顺序与预算测试；
- Markdown 预览禁用 HTML/危险链接的 GUI 测试；
- 编辑、预览、Diff 三态及窄屏切换，草稿失败恢复和快捷键测试；
- Manager HTTP、Tauri 与真实桌面纵向 smoke；
- 符号和路径扫描证明人格 `.json`、JSON parse/format、旧 persona migration、
  Governance UI 和 fallback 均未回潮。

## 当前不决定

- Agent 自动建议人格修改的具体授权模型；
- 最终 Persona authority 目录名；
- CodeMirror 6 的具体扩展集合与主题细节；
- 历史版本的长期保留数量；
- Markdown 模板是否提供默认章节。

这些问题不得被实现者自行扩展为兼容层或第二套人格系统。

## 被取代的依据

以下材料保留为历史证据，但与本决策冲突的部分不再作为实施依据：

- `docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md`
- `docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md`
- `docs/logs/2026-08-laputa-persona-workspace/`
- `agent-diva-laputa/src/persona_retire.rs` 所代表的旧文件退休/迁移模型

早期文档中的 Markdown 编辑器原则可以复用；14-section、Memory 混入人格、隐式
Proposal apply、无 Diff 和旧文件兼容不能复用。
