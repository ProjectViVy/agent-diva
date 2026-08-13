# Summary — COGNITIVE-R0 当前系统盘点

- 版本：`v0.0.1-research-package`
- 日期：2026-08-13
- 范围：文档-only 研究交付

## 变更

在 `docs/research/cognitive-r0-current-state-2026-08/` 交付 R0 完整研究包：

| 文件 | 内容 |
| --- | --- |
| `README.md` | 索引、一句话结论、Gate 自检 |
| `current-state-map.md` | 混域链、模块、读写时序、事件四轨、Prompt、GUI |
| `dependency-and-data-inventory.md` | 路径/schema、写入者、符号/路由、KEEP/RENAME/DELETE/DECIDE |
| `legacy-failure-baseline.md` | `[object Object]`、Evolution 加载失败、双账本 approval not found |

同步：`docs/research/README.md`、EPIC R0 进展、`docs/architecture/README.md`、
R1 依赖注记、`TODOLIST.md`、`docs/logs/README.md`。

## 方法

- 3 个只读子代理并行：Persona/WORLD、BML/checkpoint、Events/GUI/Approval
- 主会话交叉核对 `LaputaPaths`、`ProposalType`、`AgentEvent`、双账本路径、GUI 渲染
- 引用已有 R1 Evolution 切片，不重写 Evolution 细表
- 无生产代码修改

## 影响

- 解锁 R2 / R3 开工（R0 输入已齐）
- 不授权 D0–D4 或实施
- R4 仍等待 R2–R3
- 用户可对 R0+R1 做 Research Gate 阅读，但 Gate 通过仍需 R2–R4 齐全
