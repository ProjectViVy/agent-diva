# Release

- 本次为内部 QQ 通道稳定性修复，不涉及单独发布流程调整。
- 如需交付，沿用现有 Rust workspace 发布流程，在仓库级测试阻塞项清理后执行常规构建与发布。
- 当前未执行发布动作，因为用户未要求提交或发布，且 `just test` 仍被既有 `agent-diva-providers` 测试失败阻断。
