# v0.2.0 WorkspaceContext 工作区合同：验证记录

> 状态：骨架（Wave C1 完成后补全）

## 计划命令

- `cargo test -p agent-diva-core -p agent-diva-cli`
- `just fmt-check && just check && just test`

## 结果

- [ ] `resolve_workspace` 优先级与 canonical 化测试通过。
- [ ] LegacyDefault 迁移提示可观察（warn 日志 + doctor）。
- [ ] 外部 workspace 启动不再写入模板文件。
