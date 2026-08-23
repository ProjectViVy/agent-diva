# v0.3.0 AGENTS.md 注入 MVP：验证记录

> 状态：骨架（Wave C2+C3 完成后补全）

## 计划命令

- `cargo test -p agent-diva-agent -p agent-diva-tools -p agent-diva-cli`
- `just fmt-check && just check && just test`

## 结果

- [ ] Shell 越界 `working_dir` 负向测试转正并通过。
- [ ] WorkspaceInstructions 单元矩阵（存在/缺失/空/超限/digest/截断）通过。
- [ ] 注入段包含安全合同声明且不参与任何权限判定路径。
- [ ] status/doctor 输出注入状态但不回显正文。
