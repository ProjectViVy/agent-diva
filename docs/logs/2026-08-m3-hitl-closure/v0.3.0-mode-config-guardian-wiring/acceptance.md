# Acceptance

## 用户/产品视角验收步骤

1. GUI 切到「谨慎」：所有命令（含只读）均需确认。
2. GUI 切到「智能」：只读/allow-rule 命令自动放行，危险/未知需确认。
3. GUI 切到「信任」：只读/allow-rule/未知命令自动放行，仅危险需确认（自动学习在 S4 落盘）。
4. `Never`（bypass）路径无 Guardian，行为不变。

## 验收通过标准

- sandbox 126 / tools 117 测试 + clippy 严格模式全绿。
- 生产接线已让三模式区分行为生效；真实桌面 smoke 另排人工验收。