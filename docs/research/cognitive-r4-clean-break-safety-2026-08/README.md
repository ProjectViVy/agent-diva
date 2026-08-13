# COGNITIVE-R4：Clean-break 数据安全、删除和恢复研究

- 状态：`Research Complete (package) / Research Gate Pending User Review`
- 记录日期：2026-08-13
- EPIC：`LAPUTA-COGNITIVE-WORKSPACE-RESET` → **R4**
- 性质：删除影响、保护分支协议、零残留证明目录；**不授权**目标架构、不创建保护分支、不改生产代码
- 依赖：R0–R3 包均已落盘；产品 clean-break 决策见总 EPIC 与三份 decision-record

## 一句话结论

旧 Persona JSON、`memory_md`、未 apply 的 Proposal/Memory 治理和 AutoDream 主链停读之后，
用户最容易真丢的是 **只写进 `memory_md` / 提案、从未进 BML 的正文**。`memory_add` 的
BML 记忆和 `skills/` 应留下。保护性分支必须等 D4 指定且验证过的提交再创建；当前
`ac6edeb7`（R3 文档 tip）**不是**删除前基线。现成 `just laputa-clean-break-check`
只拦 mentle，不够本 EPIC。本包不提供导入器或 runtime fallback。

## 阅读顺序

1. [clean-break-impact-report.md](./clean-break-impact-report.md) — 按数据族的损失面、一次性手抄备份、未抽样体积
2. [protection-branch-protocol.md](./protection-branch-protocol.md) — 创建时机、命名、验证、演练、失败回退
3. [deletion-proof-catalog.md](./deletion-proof-catalog.md) — 扫描族、旧测试锁、运行时观察点、误报

## EPIC 完成物映射

| EPIC 要求 | 本包文件 |
| --- | --- |
| `clean-break-impact-report.md` | 同名 |
| `protection-branch-protocol.md` | 同名 |
| `deletion-proof-catalog.md` | 同名 |

## 证据分级

| 标签 | 含义 |
| --- | --- |
| 源码事实 | 当前树 / R0–R3 已定位实现 |
| 提交事实 | git / 已落地决策 |
| 实验观察 | 本包静态对照；**无**生产 profile 抽样、无 `just ci` |
| 推断 | 由路径推导的用户损失 |
| 建议 | 研究标记，非架构批准 |

## 产品约束（研究不得推翻）

- 被删旧链路不做文件/数据库/API/GUI 兼容
- 保护分支只追溯与恢复，不是 fallback
- 禁止导入、迁移、双读、双写、启动探测
- BML 是普通长期 Memory 唯一生产权威；`memory_md` 整链删除
- Chat Approval Center 的危险工具授权必须仍可用
- 三条旧桌面症状不沿旧模型打补丁

## 研究 Gate 自检

- [x] 三份 EPIC 完成物齐全
- [x] 引用 R0–R3 与源码路径可复现
- [x] 结论区分源码事实 / 提交事实 / 实验观察 / 推断 / 建议
- [x] 未设计导入器或 runtime fallback
- [x] 未创建保护性分支；当前 HEAD 标明不是基线
- [x] 现有 clean-break 门禁能力边界写清
- [x] DECIDE 项未假装已删
- [x] 未抽样生产 profile，操作员清单已列
- [ ] 用户 Research Gate 评审（待）

## 明确不做

D4 删除切片顺序、保护基线 SHA、扫描脚本落地、创建 `protect/*` 分支、
BML Identity 行处置、WORLD/AutoDream 报告去留定稿、生产代码修改。

## 开放缺口（交给后续）

| 缺口 | 交给 |
| --- | --- |
| 选保护基线 SHA、删除提交切片 | D4 + 用户 Architecture Gate |
| BML Identity 等 kind vs Persona | D0 |
| WORLD 物理路径 / WorldGovernance | D1 |
| AutoDream 报告 / Notebook | D3 / D4 |
| 扩 `check_laputa_clean_break.py` | D4 实施 |
| 用户机器真实体积抽样 | 操作员；发布前手顺 |
| 真机恢复演练 | I0 之后 |
