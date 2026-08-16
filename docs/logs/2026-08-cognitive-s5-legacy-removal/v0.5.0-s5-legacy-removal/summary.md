# S5 卸旧（physical legacy removal）迭代总结

- 日期：2026-08-16
- 切片：COGNITIVE-I1-S5（D4 §3.1 扫描族 + R4 §2 放大范围）
- 分支：`agent-diva-pro`（未 push）
- 版本目录：`docs/logs/2026-08-cognitive-s5-legacy-removal/v0.5.0-s5-legacy-removal/`

## 目标

按已批准的 D4 §3.1 交付计划，物理删除认知工作区 clean-break 前的旧治理面，
使 D4 扫描族在生产路径零命中（docs/research 与 offline 迁移工具除外）。
S5 只拆旧、不建新；`just cognitive-clean-break-check` 证明门与桌面 smoke 属 S6。

## 提交清单（5 笔，未 push）

| Commit | 内容 |
| --- | --- |
| `710e7684` | feat: demolish legacy persona governance GUI surfaces - 删旧 JSON 编辑器、Persona 右栏治理、混域 Inbox 等孤儿组件与绑定；重写锁旧契约的 locale 测试（断言旧 key 已不存在） |
| `677921dd` | feat: remove legacy laputa governance HTTP and Tauri surface - Manager `laputa_routes` 收缩为 recall-feedback；`handlers/bml.rs` 删除；state 移除 `laputa`/`memory_governance`/`memory_authority_mode`；ApprovalDomain::Memory 删除；Tauri notebook/proposal 命令面收缩；路由测试断言 12 个旧 URI 返回 404 |
| `5a2c1858` | feat: retire autodream legacy proposal path and workspace bml api - AutoDream `execute()` 无 MemoryHome 时 fail-closed（新错误串），只保留 skill reflection + ACTMEM Work 组织；删旧 outputs/candidates 模块与 laputa_sections 输入源 |
| `4fe49a7f` | feat: physically remove the laputa legacy kernel and mixed-domain contracts - 删 laputa 12 个 section/proposal 内核模块与 `governed_apply`、`persona_retire`、WorldGovernance 队列、core `MemoryManager`/`MEMORY.md` 权威链、混域 `ProposalType`/changelog/audit 类型、`MemoryPropose`/`MemoryApply` 能力、`authority_mode` 配置、memory 模板播种、persona-retire CLI；重写锁旧契约的测试（world_claim_routing 翻转为走 memory_add 且 `.laputa/cognitive` 不创建） |
| `1ea54d08` | docs: scrub legacy family wording from surviving modules - 幸存模块文档与测试用例脱敏，使 S6 扫描门可以严格化 |

## 保留面（按 D4 §3.2，未删）

- `agent-diva-migration` offline 导入工具及其旧格式词汇。
- `/api/approvals` 命令/计划审批域。
- 打包内置 skills 与 SkillHome 体系（S4 产物）。
- BML（`.laputa/memory.sqlite3` typed store）表结构。
- `just laputa-clean-break-check` 既有门。

## 残留与豁免说明

- 生产路径 grep 扫描中残留命中均为有意保留：迁移工具导入词汇、`.persona-workspace`
  CSS 类名、`upload_skill_zip`（S4 SkillHome 语义）、`working_memory`（BML 记录种类）、
  证伪测试中的旧符号字面量。
- `TypedMemoryStore::put_governed`/`rollback_governed` 成为无调用方的存储内部接缝
  （表结构必须保留），已记 `TODOLIST.md` `BML-GOVERNED-SEAM-DEAD-CODE`。
