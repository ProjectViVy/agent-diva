# 提案 03：整合 GUI 前端双审批 SSE 通道与重复数据源

## 1. 残留代码现状分析

### 1.1 涉及文件与位置
- `agent-diva-gui/src/App.vue` (L2000-L2020)
- [`TODOLIST.md`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/TODOLIST.md#L307-L312)

### 1.2 背景与问题点
在之前的敏捷迭代中，统一审批中心引入了统一的 `approval-event` SSE 事件流与 `unifiedApprovals` 前端数据模型。

然而目前前端代码 `App.vue` 中：
1. 仍保留了旧版的 `command-approval-requested` SSE 事件监听。
2. 前端响应式状态中同时维持着 `commandApprovals` 数组与 `unifiedApprovals` 数组。
3. 这种双通道并存的设计容易导致审批事件重复推送、卡片刷新不同步或漏掉超时倒计时。

---

## 2. 拟定的重构与瘦身方案

### 2.1 变更内容 [MODIFY & DELETE]

1. **`App.vue`**：
   - 彻底移除对旧 SSE 事件 `command-approval-requested` 的 `EventSource` 监听函数。
   - 移除前端 ref 对象 `commandApprovals`，所有审批列表统一由 `unifiedApprovals` 驱动。
   - 重构前端审批状态更新逻辑，统一走统一契约 handler。

2. **`src-tauri` 后端桥接**：
   - 确认嵌入式控制面仅广播统一 `approval-event`，清理废弃的旧版 Channel 挂钩。

---

## 3. 收益与风险评估

- **预期收益**：消除前端审批卡片丢事件/重复事件隐患；减少 App.vue 前端约 150 行杂乱的兼容代码。
- **风险分析**：中等。需要对 Tauri 前端界面进行冒烟测试（切换命令/计划审批）。
- **验证方法**：运行 `cd agent-diva-gui && npm run test:unit && npm run build`。
