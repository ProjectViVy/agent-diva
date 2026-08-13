# COGNITIVE-R3：Persona 文档权威与工作区技术研究

- 状态：`Research Complete (package) / Research Gate Pending User Review`
- 产品现行权威名：见 [`../persona-markdown-clean-break-2026-08/decision-record.md`](../persona-markdown-clean-break-2026-08/decision-record.md) P1（七份：`IDENTITY.MD` 含身体 / `RELATIONSHIP.MD` / `REDLINE.MD` / `USER.MD` / `DREAM.MD` / `DARK.MD` / `WORLD.MD`）。本包表格里的 Commitment / Preferences 是**当时代码对象**，不是现行产品名。
- 记录日期：2026-08-13
- EPIC：`LAPUTA-COGNITIVE-WORKSPACE-RESET` → **R3**
- 性质：专项研究事实、选项与 GUI 适配证据；**不授权**目标架构定稿或代码实施
- 依赖：全量 R0 [`../cognitive-r0-current-state-2026-08/README.md`](../cognitive-r0-current-state-2026-08/README.md)；
  产品边界 [`../persona-markdown-clean-break-2026-08/decision-record.md`](../persona-markdown-clean-break-2026-08/decision-record.md)；
  Frozen Core 在 C1 的位置见 R2 [`../cognitive-r2-stm-context-2026-08/context-assembly-constraints.md`](../cognitive-r2-stm-context-2026-08/context-assembly-constraints.md)

## 一句话结论

今天没有 Persona/WORLD Markdown 文档工作区。四份人格仍是预种子 `null` 的 JSON
section + `EvolutionProposal` 治理链；`content_version` 只是展示用 SHA；changelog
不是永久历史；`unified_diff` 不是逐行 Diff。WORLD 已是独立 Markdown claims，但
`WorldStore::project` 无生产 Prompt 调用者，且启动种子让 P10 的「五文件全缺」
永远不成立。GUI 是 textarea + JSON 门 + 永久右栏审批。本包写清事实与选项，
**不选**目录、Diff 引擎、CM6 扩展或 D1 方案。

## 阅读顺序

1. [persona-authority-inventory.md](./persona-authority-inventory.md) — 五权威、Frozen Core、保存链、WORLD、三套首次引导、changelog、GUI 三栏
2. [revision-diff-options.md](./revision-diff-options.md) — revision / 快照 / Diff / CAS / 历史 / 五文件原子 / WORLD store / 并发；不选赢家
3. [markdown-workspace-technical-evaluation.md](./markdown-workspace-technical-evaluation.md) — 编辑器/预览/Diff/历史/草稿/窄屏/安全渲染；可复用 vs 专用

## EPIC 完成物映射

| EPIC 要求 | 本包文件 |
| --- | --- |
| `persona-authority-inventory.md` | 同名 |
| `revision-diff-options.md` | 同名 |
| `markdown-workspace-technical-evaluation.md` | 同名 |

## 证据分级

| 标签 | 含义 |
| --- | --- |
| 源码事实 | 当前树可定位实现 |
| 提交事实 | git / 已落地决策记录 |
| 实验观察 | 本包静态 `rg` / 对照测试阅读；无新桌面复测、无活体 LLM |
| 推断 | 由事实推导 |
| 建议 | 研究标记，非架构批准 |

## 产品约束（研究不得推翻）

- 人格正文唯一权威格式是 Markdown；禁止 `serde_json::Value` / JSON patch 表达正文
- 用户直编直接保存；Agent 建议走 Persona 专用四态；不进 Approval Center
- 左导航 + 单一中央三态；删除永久右栏
- 保存带 base revision/CAS；冲突保留草稿
- 首次初始化 absence-only、一次原子直写五份权威 + 首批历史
- 每次真实成功变化永久追加完整快照 + 文本 Diff；完整历史不进 Prompt
- WORLD 参加首次引导，但是独立 claim 权威，不得作为第五个 Frozen Core 整体注入
- Clean break：不读旧 JSON section、旧 SOUL/IDENTITY/USER、persona-retire 迁移

## 研究 Gate 自检

- [x] 三份 EPIC 完成物齐全
- [x] 引用源码路径与符号可复现
- [x] 结论区分源码事实 / 提交事实 / 实验观察 / 推断 / 建议
- [x] P1–P12 产品约束未被重开
- [x] Research Hold（目录名、CM6 扩展、待审队列、Agent 提议时机、revision 物理方案、WORLD store）全部出现且未默认选择
- [x] WORLD 投影与 WorldGovernance 写入者有独立事实节
- [x] 可复用文档基础设施 vs Persona 专用状态已拆开
- [x] R0 被引用，不重写全景
- [x] 未解决问题显式交给 D1 / R4 / D0
- [ ] 用户 Research Gate 评审（待）

## 明确不做

目标 DTO/schema、Persona 目录名定稿、Diff 引擎定稿、CodeMirror 扩展/主题、
Agent 提议时机、待审队列策略、保护性分支、生产代码修改。

## 三套不得再混名的对象

| 本包称呼 | 当前代码对象 | 产品身份 |
| --- | --- | --- |
| JsonSectionAuthority | `.laputa/sections/{identity,relationship,commitment,preferences}.json` | DELETE；≠ Markdown 权威 |
| DisplayDigest | `content_version` / `authority_versions` | 展示哈希；≠ 写路径 CAS |
| GovernanceChangelog | `changelog/<id>.json` + 30 天 rollback | 提案审计；≠ 永久文档历史 |
| NaivePatchDump | `unified_diff` | 假 unified；≠ 逐行文本 Diff |
| WorldClaimFile | `cognitive/WORLD.MD` + `WorldStore` | KEEP 概念；物理与投影 Hold |
| PromptFirstRun | `FIRST_RUN_ONBOARDING_BLOCK` | DELETE；≠ 五权威原子初始化 |
| WelcomeFlag | `WELCOME_STORAGE_KEY` | 技术向导；≠ 人格完成 |

## 开放缺口（交给后续）

| 缺口 | 交给 |
| --- | --- |
| 选 revision store / Diff 引擎 / 目录名 / CM6 扩展 | D1（需 Research Gate + Architecture Gate） |
| Agent 提议时机与待审队列 | D1 |
| `WorldStore::project` 是否进 Prompt、插入点 | D1（C1 顺序约束见 R2） |
| WorldGovernance / MEMRULES 是否留在 Persona 面 | D1 / D0 |
| 用户机器真实 `.laputa/sections` / changelog 体积与备份 | **R4 已交付**手顺；本机未抽样 |
| BML `Identity` 等 kind 与 Persona 双权威 | D0 |
| 保护性分支基线与删除切片 | **R4 协议已交付**；SHA/切片交 D4 |
| 真机三态 / Diff / 初始化桌面 smoke | Architecture Gate 后的验收 |
