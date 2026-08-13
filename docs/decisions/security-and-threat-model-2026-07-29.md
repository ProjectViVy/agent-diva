# 安全与威胁模型基线

- 原始记录：`docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/legacy-docs/security/content/threat-model.md`
- 日期：2026-07-29
- 状态：`Security Baseline / Keep as Reference`

## 保留原则

- 默认拒绝未知或无法分类的高风险动作；
- 授权必须绑定能力、资源、内容摘要、策略版本和生命周期，不能跨内容或版本复用；
- 原始敏感 payload 不进入通用治理账本；领域拥有自己的数据和恢复责任；
- 失败关闭、过期、撤销、幂等和并发冲突必须是可观察、可审计的运行时结果；
- GUI 的可见性、用户在线状态和提示词承诺不能单独构成授权。

## 当前关系

危险工具授权以 `docs/architecture/current/runtime-approval-boundary-2026-08.md` 为当前
入口。Memory CRUD、Persona 初始化、STM 日常维护和 Persona 内容审查不应被此安全基线
错误地包装成聊天审批。
