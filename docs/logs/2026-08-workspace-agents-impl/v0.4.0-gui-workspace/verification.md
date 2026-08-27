# v0.4.0 GUI/Manager 工作区一致性：验证记录

> 状态：骨架（Wave D 完成后补全）

## 计划命令

- `cargo test -p agent-diva-manager`
- GUI：`npm run test`（vitest）+ `npm run build`（vue-tsc）
- GUI 冒烟：`just start` 或等效启动流程（gui-changes-need-gui-smoke 规则）
- `just fmt-check && just check && just test`

## 结果

- [ ] Manager 状态端点字段测试通过。
- [ ] GUI vitest 全绿且 vue-tsc 构建通过。
- [ ] GUI 冒烟：WorkspaceChip 显示、Workspace 设置页、切换流程、阻止半切换。
