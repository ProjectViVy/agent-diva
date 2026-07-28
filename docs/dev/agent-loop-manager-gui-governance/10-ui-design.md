# GUI 与交互治理设计

## 1. 用户体验保持

本计划不重新设计视觉语言。以下旅程必须保持：

- 启动、gateway readiness、断线恢复；
- 新建/切换/删除/恢复 session；
- 流式文本、思考、tool start/delta/finish；
- stop；
- Plan 草稿、等待审批、批准/拒绝、执行进度；
- Provider/config/settings；
- Persona/Laputa/Evolution/Notebook 现有入口；
- desktop pet、tray、prefs 和本地资产。

“DEFERRED”只允许灰显或明确不可用，不能用假数据填充。

## 2. 信息与状态边界

| 状态 | Owner | 示例 |
| --- | --- | --- |
| Domain fact | Manager/domain store | session transcript、plan phase、approval revision |
| Runtime progress | Agent event projection | token delta、tool running、compaction |
| UI in-flight | GUI composable | submitting、refreshing、stop requested |
| UI preference | Tauri LOCAL | theme、language、panel width |
| Host lifecycle | Tauri LOCAL | window、tray、embedded gateway |

GUI reducer 可以暂存 runtime progress，但 terminal state 必须由 authoritative projection 或明确 committed event 收口。

## 3. `App.vue` 目标

[App.vue](../../../agent-diva-gui/src/App.vue:192) 最终只负责：

- 装配 composables；
- 提供 top-level route/navigation；
- 把渲染模型和 intent handler 传给 view；
- 注册/释放 app-level lifecycle。

禁止继续加入 provider parsing、session cache、plan transition 或 transport 代码。建议目标：

- `<script setup>` 生产逻辑低于约 600 行；
- 单个 helper 低于约 80 行；
- 无直接 `invoke/fetch`；
- 无 DTO shape normalization。

这些数字是 review 门槛；若职责已经清晰，可带理由例外。

## 4. Chat 交互

`ChatView` 接收：

- `messages` 渲染模型；
- `streamState`；
- `planProjection`；
- `connectionState`；
- `send/stop/approve/deny/retry/selectSession` intents。

它不维护另一份 session/plan truth。乱序事件由 `useChatStream` 以 `request_id/tool_call_id/sequence` 幂等折叠。

Stop 后：

- 立即显示 stopping；
- 可移除空 placeholder；
- 迟到 delta 不再追加；
- 已完成的 tool finish 可作为证据显示，但不得重新打开 turn。

## 5. Plan/HITL

Plan 和后续 HITL 共用交互原则：

- 卡片展示对象 ID、revision、状态和风险；
- submit 时携带 `expected_revision`；
- stale response 明确提示刷新；
- approve/deny 成功后重新查询；
- unattended 模式不能替用户回答；
- UI disable 是预防，后端 policy 仍是权威。

这吸收 deep 的 interaction-surface 原则，但不回迁其 interaction crate 或 Manager v1。

## 6. Capability ledger

`src/api/capabilities.ts` 只记录真实接线状态：

```text
MANAGER   当前 Manager 业务能力
LOCAL     Tauri Host 本地能力
DEFERRED  尚未实现，调用抛出明确 not_available
REMOVED   已裁决删除，禁止再引用
```

状态变化必须伴随 test 和相关 UI 行为。不得通过 registry 创建第二套权限系统；它只是产品接线清单。

## 7. 可访问性与反馈

- loading/error/success 不只依赖颜色；
- 交互目标至少 44px；
- approval/deny 有键盘焦点和可读 label；
- 断线、stale、outcome unknown 使用不同文案；
- 长 tool args/output 安全换行并默认折叠；
- 不把内部 protocol、secret 或本地路径直接显示给用户。
