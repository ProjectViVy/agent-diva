# Channel settings recovery acceptance

日期：2026-09-04

## 自动验收

- 工作区 fmt/clippy/test、频道 clean-break、GUI dependency policy、pnpm frozen install、
  GUI tests/build、Tauri tests、Rust 1.80 channel check 与 CLI help 均已通过。
- 独立代码审查最终为 `APPROVE (P0=0, P1=0)`。
- 本地浏览器可从主页进入“设置 → 频道”，页面可交互且无 Vite overlay；无 Tauri Host 时
  显示明确可重试错误态，不伪造频道数据。

## 桌面人工验收步骤

1. 以 Tauri 桌面模式启动应用，进入“设置 → 频道”，确认现有频道卡片与 runtime 状态加载。
2. 打开新增/编辑向导，填写一个测试账号候选凭据；点击“测试连接”，确认请求不先保存凭据，
   成功显示 receipt 摘要，失败显示稳定错误并允许用户明确选择继续保存。
3. 保存后刷新，确认卡片配置和 runtime 状态一致；重复点击测试/保存不会产生重复事务。
4. 删除频道并确认，检查凭据被重置、`enabled=false`、`channels.removed` 包含频道名，刷新与
   重启后卡片仍隐藏。
5. 重新从向导配置该平台，确认 tombstone 被移除且频道可重新出现。

## 尚未满足的产品验收

本轮环境没有真实频道凭据，因此没有关闭 C5/C6-E：仍需至少一个真实平台完成入站、最终
receipt，并在切换后的生产路径复测桌面断线恢复。broad 全工作区 Rust 1.80 证明也仍开放。
