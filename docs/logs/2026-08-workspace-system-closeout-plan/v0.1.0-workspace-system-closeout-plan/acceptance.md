# Acceptance

## 规划验收

- 根 `TODOLIST.md` 明确标注 WORKSPACE 系统“已授权开工”。
- 每个 WS 切片都有范围、依赖、日期和可判断的出口条件。
- GUI workspace 展示、AGENTS 状态、原子切换、会话持久化合同和历史树均在同一 WBS，
  不再由互相割裂的待办跟踪。
- 排期保留一个风险缓冲日，但缓冲不得用于追加新功能。

## 最终产品验收门槛

1. GUI 顶部与设置页显示同一个运行时 workspace 和 AGENTS 状态。
2. 工作区切换时，UI、配置、runtime、Shell、Plan、Session 与 AGENTS 绑定到同一 root；
   失败后没有半切换。
3. 切换后只加载目标 workspace 的会话，旧 workspace 数据不迁移、不混入。
4. 历史会话按 workspace、channel 和真实 lineage 分层；legacy 会话保持可访问且不伪造关系。
5. 自动化门禁和 Windows 桌面真机 smoke 全部通过，TODO 与迭代日志完成归档。
