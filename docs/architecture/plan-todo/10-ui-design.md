# UI 设计

## 入口与布局

将现有 `agent / plan / ask` 入口，[App.vue](agent-diva-gui/src/App.vue:34)，改为“请求规划”按钮；随后 UI 永远显示后端 `PlanModeState`，不由本地按钮决定事实。

```text
┌ Plan workspace ───────────────────────────────────────┐
│ 状态：调研中（只读）  可用：浏览/搜索/读取              │
│ 目标、范围、已调研文件、假设与风险                      │
│ [继续调研]                                              │
├ 完整计划（待审时可见）─────────────────────────────────┤
│ 步骤 | 涉及文件 | 验证方法 | 风险 | 是否生成 TODO       │
│ [退回修改]                           [批准并开始执行]   │
└────────────────────────────────────────────────────────┘
```

## 交互规则

- `Exploring/Drafting`：显示“只读调研中”，明确禁止修改文件；不展示执行进度条。
- `AwaitingApproval`：显示冻结 revision、完整计划 diff/摘要、批准/退回；批准按钮可选“生成执行 TODO”。
- `Executing`：展示步骤与 TODO；TODO 是执行进度，不是计划质量门槛。
- `Verifying`：展示验证命令/证据/结论；失败提供“回到计划修订”。

openakita 的浮动进度条适合 `Executing`，[README.md](../../../.workspace/openakita/README.md:153)，但不应在调研或待审阶段制造“已执行”的错觉。现有 `approvePlanExecution` 是主要接线点，[App.vue](agent-diva-gui/src/App.vue:775)。
