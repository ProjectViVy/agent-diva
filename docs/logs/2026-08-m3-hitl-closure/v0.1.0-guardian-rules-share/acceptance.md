# Acceptance

## 用户/产品视角验收步骤

1. 无用户可见改动：本 slice 仅新增 Guardian 规则共享能力，不改变任何命令执行行为。
2. 后续 S3 接线后，Guardian 判定"已知安全"将复用 coordinator 的 `CommandRuleStore`
   规则（allow 规则自动放行）。

## 验收通过标准

- sandbox 121 测试 + clippy 严格模式全绿。
- 确认无生产行为变更（Guardian 未接线）。