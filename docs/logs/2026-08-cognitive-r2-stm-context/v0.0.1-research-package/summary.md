# Summary — COGNITIVE-R2 STM 与上下文分层

- 版本：`v0.0.1-research-package`
- 日期：2026-08-13
- 范围：文档-only 研究交付

## 变更

在 `docs/research/cognitive-r2-stm-context-2026-08/` 交付 R2 完整研究包：

| 文件 | 内容 |
| --- | --- |
| `README.md` | 索引、三套拆名、Gate 自检 |
| `context-assembly-constraints.md` | C1–C5 / Session / Plan / cron / subagent 事实与插入约束 |
| `stm-options-and-experiments.md` | Research Hold 选项与静态实验；不选赢家 |
| `stm-failure-and-concurrency-matrix.md` | 可见性 / GC / 写入 / 并发 / Prompt / 治理交叉 |

同步：`docs/research/README.md`、EPIC R2 进展、`docs/architecture/README.md`、
上下文运行时边界指针、R0 开放缺口、`TODOLIST.md`、`docs/logs/README.md`。

## 方法

- 4 个只读子代理并行：C1–C5 装配、Session/BML/checkpoint、Plan/cron/subagent、GA/Garden 对照
- 主会话交叉核对并写稿；静态 `rg`：无 `struct Stm`；`run_startup_gc` 仅定义
- 无生产代码修改

## 影响

- 解锁用户对 R2 的 Research Gate 阅读
- 不授权 D0–D4 或实施
- R3 / R4 仍待完成；R4 仍 `blocked:R0-R3`
