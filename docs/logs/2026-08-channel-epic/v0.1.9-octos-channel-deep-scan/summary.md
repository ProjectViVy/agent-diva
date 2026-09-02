# C5-P2 Octos 六频道端点深扫与实施交接补强

## Scope

本迭代只更新 C5 迁移文档和交接材料，不修改 Rust、GUI、Manager 生产路径，不执行 C6
cutover。对 Octos 固定 SHA `5ea987813de4fd2afdd1d78f2106ad2868f0d923`，为 Telegram、Discord、
Feishu/Lark、DingTalk、Email、QQ 各形成独立端点级扫描报告。

## Deliverables

- 六份 `platforms/*-scan.md` 独立事实报告。
- `endpoint-ledger.md`：HTTP/WS/IMAP/SMTP 端点和函数级映射。
- `cross-cutting-gap-matrix.md`：图片识别、群聊、审批、权限、媒体、去重、可靠性差距。
- `decision-log.md`：Port/Adapt/Retain-DIVA/Reject/Blocked 决策。
- `agent-task-cards.md`：一频道一 agent 的扫描、实现、测试、ownership、波次和合并顺序。
- `evidence-manifest.md`：target capability 到 fixture/test/receipt 的追踪骨架。
- 既有 C5 README、scan playbook、provenance、matrix、ADR、WBS、TCK、risk 和 TODO 同步。

## Handoff truth

C5-P2 文档完成不等于 C5-I/V 完成。QQ intents、group send 和 media 仍为 Blocked；六个原生
adapter、offline evidence、QQ live smoke 和 C6 clean break 继续留在 backlog。
