# UI 设计

## 1. 产品入口

Evolution 页面改为“成长工作台”，不再以多个孤立模块展示。顶部只提供：

- `立即反思`
- 自动触发状态
- Memory authority 健康
- 最近一次成功闭环时间

## 2. 主流程布局

左栏是 run 时间线，中栏是候选/提案 inbox，右栏是证据、diff、治理和最终 Memory 结果。用户从任一 run 都能追踪：

```text
输入覆盖 → 反思 → 候选 gate → proposal → decision → revision → recall feedback
```

## 3. 状态

- `ready`：可触发。
- `running`：展示当前 stage、耗时、取消。
- `no_candidates`：正常完成，解释为何没有值得记忆的内容。
- `degraded`：provider/store/workspace/预算原因和修复动作。
- `needs_attention`：恢复或治理冲突。
- `completed`：显示 proposal 数、applied 数和 revision。

## 4. 安全交互

- 默认按钮是“批准并应用”，但必须显示目标、证据、风险和 diff。
- 批量批准只允许同风险、同 scope 且均通过 quality gate 的低风险候选。
- 高/关键风险禁止一键批量。
- 拒绝时可选“仅本次”或“抑制相同候选 30 天”。
- 回滚展示影响 record/revision，重复回滚明确拒绝。

## 5. 清理

彻底隐藏或删除无真实后端的旧功能入口；不再用“即将推出”掩盖运行时空路径。月报是报告能力，不放在核心进化主流程中。
