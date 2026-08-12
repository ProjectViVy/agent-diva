# Summary

本次仅记录 STM 破坏性修复边界和调研门，没有修改产品代码。

- BML typed records 固定为普通长期 Memory 的唯一权威；
- 非兼容删除 `memory_md` 文件型/section 型长期记忆及全部生产入口；
- STM 定义为 Agent 自动维护、workspace/profile 级、跨 session、有界的活动工作集；
- 现有 `working_memory` 是 session checkpoint，不等于 STM，后续必须拆类型和命名；
- STM 更新和用户修正不走 Proposal、Governance 或 Approval；
- STM 唯一 GUI 入口归 Memory 页面，Persona/Evolution/Notebook 不再暴露 `memory_md`；
- 存储、schema、自动更新、并发、预算淘汰、历史、晋升和上下文装配进入 Research Hold。
