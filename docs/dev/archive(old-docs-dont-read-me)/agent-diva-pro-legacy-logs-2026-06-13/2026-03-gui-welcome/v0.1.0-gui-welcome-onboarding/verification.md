# 验证

## 已执行

- `pnpm run build`（在 `agent-diva-gui` 目录）：`vue-tsc --noEmit` 与 `vite build` 通过。

## 工作区全量 CI（未在本次环境完成）

- `just fmt-check`：仓库内其他 crate 存在既有 rustfmt 差异（非本改动引入）。
- `just check`：`agent-diva-gui` 的 `process_utils.rs` 等与本次无关的 clippy 告警。
- `just test`：构建阶段报告磁盘空间不足（os error 112），无法完成全量测试。

## 建议的手动冒烟（Tauri）

1. 清除或新建配置环境后删除 `localStorage` 中 `agent-diva-welcome-v1`，确认向导再次出现。
2. 在向导与设置「网络」页点击外链，确认在**系统默认浏览器**中打开，主窗口 URL 不变。
3. 向导中填写 Key 并完成，确认配置写入且无重复弹窗。
