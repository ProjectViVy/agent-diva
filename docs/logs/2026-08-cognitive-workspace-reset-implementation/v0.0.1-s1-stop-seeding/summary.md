# S1 停止旧认知种子

本批完成 D4 I1-S1：`LaputaStorage::open` 不再创建 Frozen Core 的
`sections/*.json` `null` 文件，也不再预写 `WORLD.MD` 的 `# WORLD` 标题。

保留默认 `MEMRULES.MD` 种子、基础目录布局和既有文件保护语义。S1 不删除、
迁移或覆盖旧磁盘数据，不引入 Persona 三态、首次引导或新的 API。

代码提交：`559faa8d`（`refactor: stop legacy cognitive seed writes`）。
