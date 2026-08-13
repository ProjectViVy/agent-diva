# agent-diva 文档总入口

> 更新日期：2026-08-13
>
> 当前唯一架构锚定是 2026-08-12 至 2026-08-13 的决策记录。其余资料按“当前架构、
> 当前/历史调研、关键决策、验证日志、工程参考、历史归档”分类。历史文档不删除，
> 但不再混在当前入口中。

## 阅读顺序

1. [当前架构](./architecture/README.md)
2. [关键决策集合](./decisions/README.md)
3. [当前研究与未施工研究](./research/README.md)
4. [8 月验证日志](./logs/README.md)

## 目录结构

| 目录 | 用途 | 当前规则 |
| --- | --- | --- |
| `architecture/` | 当前架构边界和运行时合同 | 只放当前入口，不放旧稿 |
| `decisions/` | 仍有方向性、范围或安全价值的决策摘要 | 8/12–8/13 架构记录优先 |
| `research/` | 当前研究、Research Hold、历史调研和论文 | 调研全文保留，按 `historical/` 分类 |
| `logs/` | 2026-08 迭代与验证证据 | 8 月原地保留；8 月前已压缩归档 |
| `engineering/` | CONTRIBUTING、项目上下文、CHANGELOG | 工程参考，不作为架构决策 |
| `resources/` | 图片等非文档资源 | 不参与文档阅读链 |
| `dev/archive(old-docs-dont-read-me)/` | 旧架构、设计、PRD、Sprint、报告、UX、开发包和旧日志 | 只用于历史追溯，附 ZIP 与 manifest |

## 8/12–8/13 锚点

- Cognitive Workspace Reset：统一 Persona、Memory/BML、STM、Evolution/Skill 和 Chat Approval 边界。
- Persona Markdown Clean Break：Markdown 权威、专用内容审查、五文件首次初始化、完整历史。
- STM Clean Break：BML 唯一长期 Memory，STM 与 SessionCheckpoint 分离，具体存储/装配仍 Research Hold。
- Evolution Reset：删除旧 AutoDream–Evolution 混合链路方向，GenericAgent 研究完成前不定稿。

## 归档规则

- 8 月前日志全部归档并压缩，8 月日志保留作当前迭代证据。
- 调研全文保留在 `research/`，旧调研进入 `research/historical/`，不删除。
- 真实且仍有方向价值的决策提炼到 `decisions/`；原始决策文件进入历史归档。
- 过期架构、设计、PRD、Sprint 计划、评审报告、提示词、UX 方案和旧开发资料进入
  `docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/` 并压缩。
- 生产源码、配置、构建文件不属于本次整理范围。
