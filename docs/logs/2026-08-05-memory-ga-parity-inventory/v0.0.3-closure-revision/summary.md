# Summary — GA-MEM-PARITY 闭环断点修订

- 版本：`v0.0.3-closure-revision`
- 日期：2026-08-06
- 类型：设计闭环审查修订（docs only，无代码变更）
- 前置：`v0.0.1-gap-inventory`、`v0.0.2-decision-freeze`

## 背景

用户要求审查 GA-MEM-PARITY 设计是否真正闭环。审查结论：**未完全闭环**，
存在 4 个断点，全部在本次修订中补入 inventory §10.3 并调整 Wave 归属。

## 断点与修订（用户确认）

| # | 断点 | 修订结论 |
|---|------|----------|
| G1 | Wave 依赖倒挂：distill 依赖 Wave 2 的 working checkpoint evidence | Wave 1 distill 最小版：输入=会话上下文（Action-Verified 自提）；checkpoint evidence 在 Wave 2 扩展（向后兼容） |
| G2 | 读侧闭环缺失：Typed prefetch 生产注入（D4）无 Wave 归属，「下次会话可见」承诺落空 | 新增 Wave 3 读侧验收：prefetch 生产注入 + 启动注入一致（D2/D3/D4、F4、H3）；W1-4 措辞修订为「Wave 1 仅承诺工具结果返回」 |
| G3 | 遗忘闭环缺失：tombstone 注入过滤无归属，「忘掉 X 后不再出现」验收不成立 | tombstone 注入过滤随 Wave 1 memory_remove 实施（startup/prefetch/上下文组装排除 tombstone） |
| G4 | 双写边界未定：AutoDream 候选与即时 apply 同内容重复（H5/G10） | Wave 4 AutoDream gate 同内容去重：与 applied authority 一致 → 跳过/降级；即时 apply 优先 |

## 变更文件

- `v0.0.1-gap-inventory/inventory.md`：新增 §10.3 闭环断点修订表 + Wave 计划影响
- `TODOLIST.md`：WAVE1 增加 G1/G3 修订；新增 WAVE3-MEMORY-READ-CLOSURE 条目

## 下一步

- Wave 1 按 §10.2 + §10.3（G1/G3）实施：六工具 + tombstone 注入过滤 + distill 最小版。
- Wave 3 按 G2 实施读侧闭环；Wave 4 按 G4 实施 AutoDream 去重。
- U1–U8 按「写→存→召回→遗忘→治理→蒸馏」六环验收。
