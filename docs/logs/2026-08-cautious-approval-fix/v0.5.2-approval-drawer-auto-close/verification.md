# v0.5.2 — 验证记录

## 自动验证（本机已执行）

| 项目 | 命令 | 结果 |
| --- | --- | --- |
| GUI 类型检查 | `npx vue-tsc --noEmit`（agent-diva-gui） | 通过（无输出） |
| GUI 测试套件 | `npm test`（vitest run） | 57 文件 / 447 测试全部通过 |
| Rust 工作区 | 未运行 | 本次无 Rust 变更，不影响后端 |

## 手动 GUI 烟雾测试（用户执行）

前置：关闭正在运行的 `just make-diva` 双窗口（agent-diva.exe / agent-diva-gui.exe），重新 `just make-diva`。

| # | 场景 | 预期 | 观察结果 |
| --- | --- | --- | --- |
| 1 | 谨慎模式执行命令 → Drawer 自动弹出 | Drawer 自动打开，且仅 1 张卡片（无内联/横幅） | 待填 |
| 2 | 在自动弹出的 Drawer 点击「同意」 | 命令执行成功，Drawer **自动关闭** | 待填 |
| 3 | 手动点击审批中心图标打开 Drawer，再同意 | Drawer **保持打开**（非自动弹出场景不关闭） | 待填 |
| 4 | 自动弹出后点击「拒绝」 | Drawer 保持打开 | 待填 |
| 5 | 自动弹出后刷新（F5）再触发审批 | 仍只显示 1 张卡片，同意后自动关闭 | 待填 |
| 6 | Plan 审批卡（PlanApprovalCard） | 行为不变（不随本变更关闭） | 待填 |
