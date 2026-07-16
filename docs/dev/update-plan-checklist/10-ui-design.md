# UI 设计文档

## 展示语义

普通聊天 `update_plan` 显示为只读“任务清单 / Task Checklist”，不显示“计划更新”。正式计划仍使用计划审批组件，执行期 TODO 仍保留交互式勾选能力。

## 事件交互

1. tool started 创建运行中工具行。
2. checklist update 将该行替换为只读清单。
3. tool finished 保留清单内容并创建 assistant 占位符。
4. final 定位并关闭占位符，即使其他卡片出现在其后。

## 状态视觉

`pending` 使用时钟，`in_progress` 使用旋转加载图标，`completed` 使用勾选。旧 PascalCase 状态映射到相同视觉。参见 [TodoCard.vue](agent-diva-gui/src/components/TodoCard.vue:20)。

## 可用性

清单为只读，不提供“全部完成”或单项勾选；完成后保留在聊天历史中。窄屏继续复用现有卡片响应式布局。
