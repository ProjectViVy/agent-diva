# 验收记录

## 计划验收

维护者可按以下步骤确认本次交付：

1. 打开根目录 `TODOLIST.md` 的 `HARNESS-SESSION-ADMISSION-BOUNDED-QUEUE`。
2. 确认 HQ-00～HQ-05 均有日期、工期、交付物和前后依赖。
3. 确认基线窗口为 2026-08-31 至 2026-09-16，风险缓冲最晚至 2026-09-18。
4. 确认首阶段包含 AgentLoop 并发所有权闸门，并禁止以全局 mutex 代替目标语义。
5. 确认验收覆盖 FIFO、跨 session 并行、queue full、timeout、cancel、Stop/Reset、
   idle eviction、无副作用拒绝与全套 Rust/GUI smoke 门禁。
6. 确认 EventBus Hook、MessageBus 重写、token queue、A2A 和 Neuro-Link 不在本 Epic 范围。

## 产品验收（后续 HQ-05）

本 planning 版本不宣称产品功能可用。只有 HQ-00～HQ-05 全部完成、自动化门禁通过且至少
完成 CLI/GUI/Channel 的真实入口 smoke 后，才可勾选并归档 Epic。
