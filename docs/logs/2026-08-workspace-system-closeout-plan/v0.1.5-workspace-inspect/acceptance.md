# Acceptance

- 在 Workspace Settings 输入一个存在的目录并执行“预检”，页面展示 canonical root、workspace ID、
  可读性和 AGENTS.md 状态。
- 点击原生“选择目录”后，取消选择不会改变当前 workspace；选择不可读、文件或不存在路径时，
  页面显示错误且当前 workspace 仍保持不变。
- 快速连续预检两个目录时，最终页面只显示最后一次选择的候选，不被旧请求覆盖。
- 在 WS-03 完成前，候选只停留在 draft，不得被误认为已切换。
