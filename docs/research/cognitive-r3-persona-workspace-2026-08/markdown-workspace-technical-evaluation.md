# R3 Markdown 工作区技术评估

- 状态：`Research Draft / Evaluation Only`
- 日期：2026-08-13
- 性质：对照 P3–P7 评估现 GUI / 依赖；**不定** CodeMirror 扩展集、主题或线框
- 盘点见 [`persona-authority-inventory.md`](./persona-authority-inventory.md)

## 1. 能力缺口总表

| 能力 | 产品要求 | 当前 | 缺口 | 证据 |
| --- | --- | --- | --- | --- |
| 源码编辑 | 行号、语法高亮、当前行、查找、撤销/重做、自动换行、滚动、`Ctrl/Cmd+S`、dirty | 裸 `<textarea>`；有 `Ctrl/Cmd+S` 与 dirty；**无**行号/高亮/查找/当前行 | 不满足 P7「成熟编辑器」 | `SectionEditor.vue:275-281, 155-160` |
| JSON 门 | 禁止；正文是 Markdown | `JSON.parse` 失败禁保存；「格式化 JSON」按钮；预览是 `JSON.stringify` | 与 P1/P4 相反 | `77-95, 186-191, 290` |
| 预览 | 实时人类可读 Markdown；可折叠；收起后编辑器占满 | `<1024px` 互斥 tab；`≥1024px` 双栏但两侧都是 JSON（编辑 textarea + pretty `<pre>`），tab 被 CSS 藏起；**不**调用 `markdown-it` | 无 Markdown 预览；宽屏不可收起成纯编辑 | `243-291`、`584-596` |
| Diff | 只读 before/after、同步滚动、逐行增删改 | 无。Approval 把 `proposed_patch` 当 `"diff"` 字段；changelog 用 naive dump | 无文本 Diff 组件 | `approval_service.rs`；`proposals.rs:729` |
| 待审三态 | 中央「待审变更」；接受/拒绝原子；stale 禁接受 | 右栏 `PersonaLifecyclePanel`：decide/apply/edit patch/rollback | 安全审批混入；无 stale | `PersonaLifecyclePanel.vue:1-60` |
| 历史 | 中央只读；载入=草稿；未保存确认 | 模态 50 条 changelog；只能复制 `after` | 无载入草稿、无不可变版本浏览 | `HistoryModal.vue:42, 141-151` |
| 草稿恢复 | 保存失败/409 保留正文、选区、滚动 | 失败保留 textarea（未 reset）；成功会把 draft 打回 `original`（服务端未写入） | 无选区/滚动持久化；成功路径会丢掉用户刚提交的正文 | `SectionEditor.vue:137-145` |
| 切文件丢草稿 | 动作边界提示 | `appConfirm` 存在 | 可用；需带到三态切换 | `PersonaMemoryView.vue:98-105` |
| 布局 | 左四文档 + 单中央三态；删永久右栏 | 三栏 230 / 1fr / 250；左含 memory_md/changelog/memrules/world；`≤1050px` 藏右栏 | 窄屏不是标签化单工作区 | `PersonaMemoryView.vue:182-194` |
| 首次引导 | 一次收集五权威，原子提交 | `WelcomeWizard` = 密钥/网络；Prompt 另催提案 | 两套都不是 P9–P11 | `WelcomeWizard.vue`；`WELCOME_STORAGE_KEY` |
| 危险 HTML | 预览禁原始 HTML；核链接协议与外链 | Persona 预览无 HTML 渲染（JSON pre）。Chat/Plan/Notebook/`DivaPet` 用 `markdown-it` `html:false` + `linkify:true`；未见 `javascript:` 协议过滤 | 复用预览时必须补协议白名单 | `ChatView.vue:49-64, 1029`；`openExternal.ts` 无协议检查 |
| 无障碍 | 最低：焦点、标签、键盘 | HistoryModal 有 focus trap / Escape / `aria-label`；编辑器有 tablist；右栏按钮无统一 dialog 模式 | 三态/Diff/首次引导都还没有 | `HistoryModal.vue:71-117` |

标签：上表「当前」列为源码事实；「缺口」为对照产品后的推断。

## 2. 现有 `markdown-it` 适配证据

`agent-diva-gui/package.json`：

- 有：`markdown-it@^14.1.1`、`@types/markdown-it`、`highlight.js@^11.11.1`
- 无：`codemirror`、`@codemirror/*`、`monaco-editor`、`diff`、`diff2html`

生产调用点（全部 `html: false`）：

| 调用方 | 配置 | 用途 |
| --- | --- | --- |
| `ChatView.vue` | `html:false`, `linkify:true`, `breaks:true`, hljs | 聊天气泡 `v-html` |
| `AgentMessageBody.vue` | 同上 | Plan 消息 |
| `PlanDocument.vue` | `html:false`, `breaks`, `linkify`；无 hljs | 计划正文 |
| `NotebookView.vue` | `html:false` | 笔记 |
| `DivaPetView.vue` | `html:false`, `linkify`, `breaks` | 桌宠 |

Persona **零引用**。复用预览不需要新依赖，但必须抽成共享渲染器，并补：

1. 保持 `html:false`（P7 已要求）。
2. 链接协议。`markdown-it` **默认** `validateLink` 拒绝 `javascript:` /
   `vbscript:` / `file:` / 非图片 `data:`（`node_modules/markdown-it/lib/index.mjs`）。
   GUI 未覆盖该函数，也未加更严白名单。实验观察。
3. 外链点击不走 `openExternalUrl`（该助手本身也无协议过滤，`openExternal.ts:5-11`）。
   Tauri `csp` 为 `null`。源码事实。
4. 图片：`html:false` 后 `![](url)` 仍渲染 `<img>`，可打远程。D1 需决定是否允许。
5. 不要把 Chat 的 hljs 主题强加给 Persona 预览。

建议（非批准）：预览复用 `markdown-it` 是低风险；Diff 与源码编辑不要指望它。

## 3. 编辑器库：CodeMirror 6 vs 对照

P7：实施前评估 CM6 与 Vue/Vite/Tauri；**默认优先 CM6**；完整 Monaco 仅在确需
语言服务时考虑。本包只给适配证据，不选扩展集、不选主题。

### 3.1 宿主约束（源码事实）

- Vue 3.5 + Vite 6 + Tauri 2 + TypeScript 5.6 + vitest / happy-dom
- GUI 已很重：`three`、`@pixiv/three-vrm`、`@sparkjsdev/spark`、avatar 本地包
- 无 SSR（Tauri webview / Vite SPA）

### 3.2 CodeMirror 6

| 项 | 证据 / 评估 |
| --- | --- |
| 能力拟合 | 行号、Markdown 高亮、查找、历史、换行、keymap（含 Mod-s）均是官方扩展，正好覆盖 P4 清单 |
| Vue 包装 | 社区 `@codemirror` 核心与 `codemirror` 包；也可薄包装 `EditorView`。不强制新 UI 框架 |
| 包体 | 按需扩展；典型 Markdown 编辑器远小于 three/VRM。推断：对已有 3D 包体不敏感 |
| 许可证 | MIT（官方包） |
| 测试 | happy-dom 里可挂 DOM；需避免在每个 vitest 里启动完整编辑器，用门面 mock |
| Tauri webview | 标准 DOM；无原生插件 |
| 风险 | 扩展集合一旦膨胀会变成「第二个 IDE」；P7 明确第一版不要语言服务 |

建议（非批准）：与 P7 默认一致——CM6 **适配**，扩展保持最小（history、search、
lineNumbers、markdown lang、keymap）。主题/具体包名留给 D1。

### 3.3 Monaco（对照）

| 项 | 评估 |
| --- | --- |
| 能力 | 完整 LSP/多文件 IDE；远超人格文档 |
| 包体 | 显著大于 CM6；与 three/VRM 叠加伤害首启 |
| Vue/Vite | 需 worker 配置；Tauri 打包 worker 路径是已知痛点 |
| 语言服务 | 产品不需要 JSON schema（还要删 JSON） |

建议（非批准）：**不**作为 Persona 默认。只有未来出现真正的多语言工作区才重开。

### 3.4 继续 textarea + CSS 行号

P7 明文禁止。高冲突。本包不当成合法选项。

## 4. Diff 组件

今天仓库没有行对齐 Diff UI。需要的最小能力：

- 两侧只读 Markdown（或纯文本）
- 同步滚动
- 行级增/删/改高亮
- 不提供行内编辑、不提供合并按钮以外的「接受/拒绝」（那是领域动作）

实现对照（均不选定）：

| 路线 | 要点 | 风险 |
| --- | --- | --- |
| 自绘两栏 + 服务端 unified | 少依赖；测算法即可 | 要写滚动对齐 |
| 轻量 `diff` npm + 自绘 | 浏览器算 Diff | 与存储 Diff 算法必须同版本 |
| CodeMirror Merge | 与 CM6 同源 | 容易滑向可编辑 merge view（P5 禁） |
| Monaco diff | 重 | 见 §3.3 |

文本 Diff **展示**可以被 Evolution/Memory 文档以后复用。业务状态机不得因此合并。

## 5. 可复用基础设施 vs Persona 专用

| 可复用（文档基础设施） | 必须 Persona 专用 |
| --- | --- |
| Markdown 源码编辑器外壳（CM6 门面） | `PersonaDocument` 四对象 + WORLD 首次引导语义 |
| `markdown-it` 安全预览（html off + 协议白名单） | Frozen Core 会话冻结与预算 |
| 行级 Diff 高亮 + 同步滚动 | `PersonaChangeRequest` 四态 `pending\|accepted\|rejected\|stale` |
| dirty / 丢草稿确认 / `Ctrl+S` | 用户直存 vs Agent 请求的权限差 |
| 409/stale 稳定错误码的展示（保留草稿） | 五权威 absence-only 状态机 |
| 历史列表虚拟滚动、只读渲染 | 「载入=草稿、保存=新头」的版本策略 |
| 窄屏单工作区 tabs 布局骨架 | 左栏**只**四文档（+ WORLD，若 D1 把 WORLD 放进来） |

禁止借上表左列复活：`EvolutionProposal`、`PersonaLifecyclePanel`、
Governance Ledger、Approval Center `domain=memory`、WorldGovernance 当通用
收件箱。

MEMRULES：今天只读挂在 Persona 左栏。产品左栏没有它。是否共用预览组件可以，
是否留在 Persona 导航是 D0/D1 DECIDE。

## 6. 草稿、冲突、成功路径的现坑

源码事实：`handleSave` 在 **HTTP 成功**后执行：

```text
draftContent = originalContent
emit update:modelValue(original)
清空 changeReason
emit proposal-created → 父组件 reload
```

因为写路径只建提案、不改权威，`originalContent` 仍是旧 JSON。用户刚写的正文
被打回旧值。这与 P4「成功后正文原子成为当前权威」和「失败保留草稿」都不一样：
成功也丢草稿。D1 直写 API 必须改这个时序（保存成功以服务端新头为 original）。

失败路径：`saveError` 展示，draft 不复位——这一点接近 P4。没有 409 分类。
没有光标/滚动恢复。

`pendingProposal` 只是提示，**不**阻止继续提交新提案。与 stale 规则无关。

## 7. 首次引导 UI

| 现成向导 | 收集什么 | 完成标记 |
| --- | --- | --- |
| `WelcomeWizard` | DeepSeek / Bocha 密钥与跳转 | `localStorage agent-diva-welcome-v1` |
| Prompt first-run 块 | 称呼 / 协作 / 边界 / 沟通 → 四个提案 | Frozen Core 非空 |
| 无 | WORLD 环境 | WORLD 已被种子 `# WORLD\n` |

产品要的是**一个**五栏收集 + 一个提交边界 + 服务端
`uninitialized|ready|incomplete`。现成两套都列删除范围（P11）。技术配置向导
若保留，必须与人格初始化分离——本包建议（非批准）保留 Welcome 只做密钥，
不读、不写 Laputa 权威。

## 8. 无障碍与窄屏

已有、可借鉴：

- HistoryModal：scrim 点击关闭、Escape、焦点陷阱、关闭后焦点回到触发按钮
- SectionEditor：`role=tablist` / `tabpanel`、保存/历史 `aria-label`
- 切文件 `appConfirm`

没有：

- 中央三态的 `tab` + `aria-selected` 作为**唯一**主工作区（今天 tabs 只切
  JSON 编辑/预览）
- Diff 双栏键盘滚动
- 首次引导分步的焦点顺序
- `≤1050px` 只是藏右栏，左栏仍在；更窄时 230px 左栏会挤爆中央

建议（非批准）：窄屏用产品允许的「明确单工作区标签」，不要再靠藏右栏。

## 9. 测试面（给 D1，不是本包去写）

现成锁旧合同、实施时要替换的：

- `SectionEditor.spec.ts`：合法 JSON 才提交；非法 JSON 不调用 API
- `PersonaMemoryView.test.ts`：错误条不含 `[object Object]`
- `SectionGroupList.test.ts`：左栏含 `memory_md`
- `frozen_core.rs`：`null` 种子、JSON 捕获
- `agent_loop.rs:2901` first-run onboarding 注入

新能力最低测（决策已列，此处不扩架构）：

- Markdown 往返、空文档、Unicode Diff
- 预览无原始 HTML、危险协议
- 三态切换与窄屏
- 接受/拒绝原子、stale 禁接受
- 历史载入只改草稿
- 五权威存在矩阵

## 10. 建议（非批准）

1. 预览：抽共享 `markdown-it`（`html:false` + 协议白名单）；Persona 不要 JSON 预览。
2. 编辑器：按 P7 默认评估 CM6 最小扩展；Monaco 仅对照。
3. Diff：独立只读组件，禁止可编辑 merge。
4. 右栏治理、JSON 门、changelog 模态、Welcome 当人格完成：删除/拆离。
5. 不要为了「先能用」给 textarea 画行号。

以上不是 D1 选型结论。
