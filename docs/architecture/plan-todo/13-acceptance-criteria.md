# 验收标准

## 功能

- [ ] 复杂任务进入计划后，探索与计划阶段仅能调用 `Inspect`/`PlanningRecord`。
- [ ] “完整计划”至少含目标、范围/受影响文件、步骤、风险/假设、验证方法和未决问题处理。
- [ ] `AwaitingApproval` 在任何后续普通消息下仍拒绝写入、执行、外部和子代理工具。
- [ ] 用户批准指定 revision 后才进入 `Executing`；revision 不匹配时不产生任何副作用。
- [ ] TODO 生成取决于计划/用户选择；不需要 TODO 的任务可直接执行与验证。
- [ ] 需要 TODO 时，清单从已批准步骤物化，状态更新原子、可审计、不会重复。
- [ ] 验证失败可回到可编辑计划，原批准失效并保留历史。

## 质量与安全

- [ ] 每个状态×能力组合有单元测试；未知工具默认拒绝。
- [ ] 对照 OpenHarness 的系统提示刷新回归，[test_runtime_plan_mode.py](../../../.workspace/OpenHarness/tests/test_ui/test_runtime_plan_mode.py:45)，同时验证运行时硬拦截。
- [ ] `just fmt-check && just check && just test` 全部通过，无新 warning。
- [ ] 生产代码无新增 `unwrap`/`expect`、无新增 `unsafe`、无硬编码凭据。
- [ ] 旧 Plan/TODO 可读取且不被静默删除；升级不要求用户手工修改配置。

满足以上条件才可称为“计划模式完成”：它既能充分调研和产出完整计划，又在审批之前以系统能力而不是提示词保证零工作区副作用。
