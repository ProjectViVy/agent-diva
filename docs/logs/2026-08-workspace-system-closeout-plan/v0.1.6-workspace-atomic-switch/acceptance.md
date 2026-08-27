# Acceptance

- 在无流式输出、无 Plan、无待审批和无 HITL 问题时，预检通过的候选可以进入确认按钮；切换
  成功后顶部 chip、Settings、运行时 `/api/workspace` 和历史会话都指向同一新 root。
- 任一 unsafe 状态存在时，确认按钮禁用，后端仍以 request guard 拒绝绕过 GUI 的调用并说明原因。
- 目标 runtime 启动或 workspace 校验失败时，配置、gateway 和 GUI snapshot 回到旧 workspace；
  回滚本身失败时显示可重试错误，不显示“切换完成”。
- 切换成功后旧 workspace 的本地消息和删除标记不污染新 workspace；新 workspace 仅加载自己的
  active GUI session 集合。
- debug 外部 gateway 调用切换命令时得到明确拒绝，不执行只改配置的半切换。
