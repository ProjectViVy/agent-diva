# v0.1.0 行为表征测试：验收步骤

> 状态：骨架（Wave B 完成后补全）

1. 运行 `cargo test -p agent-diva-cli -p agent-diva-agent -p agent-diva-tools`，确认
   新增表征测试全绿。
2. 确认本版本不含任何生产代码 diff（`git show --stat` 仅 tests 目录）。
