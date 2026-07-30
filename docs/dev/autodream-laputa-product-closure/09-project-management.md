# 项目管理与批次

## 1. 新的门控顺序

原 G2D 从前置 P0 调整为最终人工验收门。原因是当前只能验证提案底座，不能证明完整产品可用。自动化安全与数据完整性 gate 不后移。

```text
E0 基线
 → E1 Experience Journal
 → E2 Orchestrator
 → E3 Reflection/Candidate Gate
 → E4 Proposal/Governance
 → E5 Typed Apply/Recall Feedback
 → E6 GUI 产品闭环
 → E7 自动化全量门
 → G2D+ 真实桌面最终验收
 → Evolution 可用
```

## 2. 可提交切片

| 切片 | 主要产物 | 退出条件 |
|---|---|---|
| E0 | characterization、reason code、真实状态 UX | 不再有“假可用” |
| E1A/B | evidence 类型、journal、回填 | 脱敏、幂等、隔离 |
| E2A/B | queue/state machine、worker/recovery | 普通 trigger 真执行 |
| E3A/B | reflection trait、candidate gate | 非占位候选稳定 |
| E4A/B | publisher、统一 approval contracts | edit/reject/replay 完整 |
| E5A/B | canonical mapping、feedback loop | apply 后可 recall |
| E6A/B | run workspace、proposal/Memory result UX | 无需 API 手工操作 |
| E7A/B | drills、release gates、desktop acceptance | 安装后开箱可用 |

每个子切片都使用精确 `LOCK.md` 范围、四件套日志、聚焦提交，不 push。

## 3. 人工暂停点

只有以下事项暂停请求用户：

- 最终真实桌面点击和视觉观察；
- 真实 provider smoke 与 `keys.txt`；
- 不可逆用户数据迁移、发布、push；
- 会改变产品语义的新选择。

实现期间不再要求用户逐项 G2D，直到 E7 自动化全量门通过。

## 4. Definition of Done

“拿到手就能用”必须同时满足：

1. GUI 能触发真实 AutoDream run 并看到进度；
2. run 能从真实 evidence 生成非占位候选；
3. 候选可批准、编辑、拒绝、抑制和回滚；
4. approved 内容唯一写入 typed store；
5. 新会话能 recall，回滚后不再 recall；
6. 无 provider/无 evidence/损坏 store 时有可操作错误；
7. 重启、并发和崩溃不重复执行；
8. 自动化 E2E 与最终真实桌面验收均通过。
