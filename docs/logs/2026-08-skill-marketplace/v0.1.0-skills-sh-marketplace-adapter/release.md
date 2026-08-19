# v0.1.0 skills.sh Marketplace Adapter — Release

## 发布方式

本变更为本地网关 + 桌面 GUI 能力，发布即「重建并重启」。

1. 重建网关（manager 已编入 `agent-diva-cli`）：
   - `just build-release` 或按需 `cargo build -p agent-diva-cli --release`。
2. 重建 GUI（含 Tauri 命令与前端）：
   - 走既有 `scripts/package-windows-gui.ps1` 打包流程，或本地
     `npm run build` + `cargo build`（src-tauri）。
3. 重启网关与 GUI 使新路由/命令生效。

## 环境说明

- 默认访问 `https://skills.sh`；如处于受限网络，可用环境变量
  `AGENT_DIVA_SKILLS_MARKETPLACE_URL` 指向镜像/代理端点。
- 无需数据库迁移、无配置 schema 变更、无破坏性接口改动。

## 未推送

按仓库约定，提交仅在本地 `agent-diva-pro` 分支，未 push（用户未要求）。
