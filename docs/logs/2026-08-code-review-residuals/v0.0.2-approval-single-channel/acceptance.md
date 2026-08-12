# Acceptance

1. GUI 启动后只建立一个审批 SSE，事件只从 `approval-event` 进入 Drawer。
2. 倒计时归零只触发一次详情刷新；服务端返回 `expired` 后操作保持禁用。
3. 刷新失败时 UI 保持 `0s`、禁止批准并显示既有可恢复刷新入口。
