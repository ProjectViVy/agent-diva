# Verification

- 检查技术规格明确覆盖稳定前缀、检查点、活跃尾部、工具链配对和 artifact 一致性。
- 检查 clean-break 门禁明确禁止双写、fallback、影子路径、长期 flag 和静默截断。
- 检查 TODOLIST 已拆分 C5a–C5d，且研究 README 指向新的当前施工权威。
- 文档改动执行 `git diff --check`；无生产代码，因此不运行 Rust 构建与测试。
