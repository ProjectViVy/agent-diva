# v0.0.1 — Claude Code 审批模型调研与完善方案

## 变更内容

- 调研 Claude Code 权限/审批模型（6 种模式 + settings.json permissions 规则系统），
  盘点 agent-diva 现有审批链路（AskForApproval 4 策略 + GUI 三模式 + Guardian + 规则系统）。
- 产出调研文档：`docs/research/approval-model-claude-code-vs-agent-diva.md`。
- 结论：agent-diva 审批模型存在 7 项差距（G1-G7），其中 4 项为核心"半残"证据：
  - G1 信任≡谨慎（guardian.rs:369 / exec_policy.rs:299 同分支）；
  - G2 智能=盲跑（guardian.rs:365 OnFailure 直接 Defer，无风险预判）；
  - G3 自动放行开关生产默认全关（orchestrator.rs:885 固定 GuardianConfig::default()）；
  - G4 UnlessTrusted 无独立语义。
- 文档给出「智能、谨慎、信任」三窗口完善方案（P0/P1/P2 分阶段）。

## 影响范围

- 仅新增调研文档 + 迭代日志；无代码变更。
- 实现改动待后续迭代（见 TODOLIST `审批三模式完善`）。

## 来源

- Claude Code 官方文档（code.claude.com）、runoob/w3cschool/CSDN/腾讯云教程；
  源码行号引用见文档附录。
