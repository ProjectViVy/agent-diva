# 2026-08 全量文档整理归档批次

- 创建日期：2026-08-13
- 范围：`docs/` 中 2026-08-01 以前的日志，以及本次清理前散落的旧架构、设计、PRD、Sprint、报告、UX、提示词和开发资料。
- 原则：原文保留、按主题与时间归档、当前入口与历史资料分离；不修改生产源码、配置或构建文件。

## 归档结构

- `architecture/legacy/`：旧架构、治理合同、Memory 合同、计划和旧决策。
- `legacy-docs/`：旧设计、计划、PRD、报告、提示词、安全、UX、根目录历史文档及旧 docs archive。
- `legacy-dev/`：旧开发专题包、开发归档和过往开发资料。
- `logs/pre-2026-08/`：2026-08-01 以前的全部迭代日志；按原目录结构保留。
- `packages/`：上述归档集合的 ZIP 压缩副本及校验信息。

调研全文不放在本批次中：已恢复并统一保留在 `docs/research/`，其中较早、已完成但未施工或仅作参考的材料在 `docs/research/historical/`。研究包仍可有 Research Hold，不因未实施而删除。

## 当前替代入口

- 当前架构：`docs/architecture/README.md`
- 方向性决策：`docs/decisions/README.md`
- 当前及历史调研：`docs/research/README.md`
- 2026-08 迭代证据：`docs/logs/README.md`
- 归档文件数、压缩包大小和 SHA-256：`manifest.md`

归档中的旧结论可能曾经有效、现在已被替代或仅停留在研究阶段。除非进行历史追溯，不要将归档内容当作当前设计输入。
