# v0.0.1 real-budget-status

- 将 GUI 主聊天页底部预算圆环从硬编码 `50%` 改为基于当前会话消息和 `tools.budget` 的真实预算压力展示。
- 将“设置 -> 压缩”面板接到真实 `tools.budget` 配置，支持查看当前会话预算状态并保存 `max_tokens` / 阈值 / `keep_recent_count`。
- 扩展 manager 的 tools 配置 DTO，使 GUI 读取/保存工具配置时包含 `budget`，避免网络/mentle 设置覆盖预算配置。
- 新增前端预算计算单测，覆盖“欢迎占位消息不计入预算”和“超阈值触发压缩”两条关键行为。
