# WS-03：工作区原子切换事务

本阶段把候选确认接入停止—保存—重建—恢复流程：

- Tauri 端以串行锁保护切换；流式输出、Plan、审批和 HITL guard 任一命中即拒绝。
- 候选再次预检后，先保存旧 workspace 会话快照，再保存目标配置、停止旧 embedded gateway，
  重建完整 `GatewayRuntimeConfig`/Manager runtime，并以 `/api/workspace` 验证新 canonical root。
- 验证成功后才安装新的 gateway handle、端口和 GUI workspace snapshot；失败会恢复旧配置并
  重建旧 runtime，恢复失败则返回明确可重试错误，不伪造成功。
- GUI 在成功边界清空旧本地消息状态、刷新新 workspace session authority、恢复最新 GUI 会话
  和 active Plan；切换期间阻止新的发送操作。
- debug 模式使用外部 gateway，原子切换显式拒绝并要求重启外部 backend，避免制造半切换假象。
