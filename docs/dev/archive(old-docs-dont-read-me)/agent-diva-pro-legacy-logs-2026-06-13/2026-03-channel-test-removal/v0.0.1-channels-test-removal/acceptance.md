# Acceptance

验收步骤：

1. 搜索仓库中的 `test_connection`、`test_channel`、`channel test`、`connection test` 等关键字。
2. 确认 channels 相关生产代码与公开接口中不再提供 channel 配置测试能力。
3. 确认未误删 provider 连接测试相关 GUI 入口。
4. 运行 `cargo check -p agent-diva-channels` 并确认通过。

验收结果应满足：

- channel test 能力无残余接口、实现或误导性测试命名。
- 无 manager / cli / tauri 的 channel test 入口残留。
- 本轮改动边界未扩散到 GUI/provider/runtime 的无关改造。
