# Summary

M3 HITL 收尾 S5：GUI 审批模式（`permissionMode`）持久化到 localStorage，消除 G5
（模式不持久化，重启回默认）。

## 变更

- `agent-diva-gui/src/components/ChatView.vue`：
  - `permissionMode` 初始化从 `localStorage['agent-diva.permissionMode']` 读取，缺失
    时回退 `'smart'`。
  - 新增 `watch(permissionMode, ...)` 在变更时写回 localStorage。
- `agent-diva-gui/src/components/ChatView.test.ts`：新增测试断言恢复与持久化。

## Impact

- 用户设置的谨慎/智能/信任模式跨会话保留，重启不再回退智能。
- 无后端/API 变更。