# M3 HITL Goal 执行资料包

本目录是使用 Codex `/goal` 完成 M3（GMH-30..33）的权威执行合同。目标必须按
`00-goal-brief.md` 指定顺序推进，并在 `06-goal-checkpoints.md` 的人工门禁暂停。

阅读顺序：

1. `00-goal-brief.md`：目标、边界、完成定义与可直接使用的 `/goal` 文本；
2. `01-current-state-inventory.md`：当前实现与缺口；
3. `02-state-machines-and-recovery.md`：三域状态机、消费与恢复；
4. `03-api-event-contract.md`：Manager HTTP/SSE/Tauri 契约；
5. `04-gui-headless-design.md`：GUI 与 CLI/headless 产品行为；
6. `05-test-and-human-acceptance.md`：自动化、真实路径与人工验收；
7. `06-goal-checkpoints.md`：阶段停点、证据和续作协议。

若本文档与根 `AGENTS.md` 冲突，以 `AGENTS.md` 为准；若执行中必须改变已冻结的
公开契约或恢复语义，必须暂停 Goal，记录影响并取得人工确认。
