# Summary

对当前 Diva AutoDream 做独立测试，并订正产品口径：必须整理 STM；人格整理只允许提案。

## 改了什么

- 跑通现有 `agent-diva-autodream` 套件与 Manager `autodream_laputa_e2e`。
- 新增表征测试 `agent-diva-autodream/tests/current_contract.rs`，钉死今天的实现：
  默认读 Identity JSON + `memory_md`、没有 STM 源、默认反射发 `MemoryPatch`、
  闸门接受 `IdentityPatch`、只拒 `SopCreate`。
- 修订 P19 / STM S5 / Evolution D5 / EPIC 第 9 条 / 当前架构边界 / TODOLIST：
  AutoDream 主职是整理 STM（直写）+ 按允许表提人格案。

## 影响范围

- 测试：`agent-diva-autodream/tests/current_contract.rs`（表征，不实现 STM）。
- 文档：Persona / STM / Evolution 决策、EPIC、架构摘要、TODOLIST、本日志。
- 无生产 AutoDream / Laputa / STM 实现改动。Research Gate 仍挡住 D0–D4。
