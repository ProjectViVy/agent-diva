# v0.2.0 Featured Leaderboard Snapshot — Release

## 发布方式

随下一次 Windows 桌面构建发布（NSIS/MSI，`scripts/package-windows-gui.ps1`）。
快照为构建期内嵌数据，无需运行时配置、环境变量或网络权限。

## 运维说明

- 精选数据 = 提交在仓库中的 `agent-diva-manager/data/marketplace_featured.yaml`。
- 刷新节律（手动）：
  1. `python agent-diva-manager/scripts/fetch_marketplace_featured.py`
     （可选 `--view trending|hot`、`--limit N`；设置
     `AGENT_DIVA_SKILLS_MARKETPLACE_TOKEN` 可走官方 v1 API）；
  2. 检查 diff 合理后提交 YAML；
  3. 重新构建网关/GUI 生效。
- 降级链：有 token → 官方 `/api/v1/skills`；无 token → 首页 Leaderboard
  HTML 解析。两者失败则脚本报错退出，不覆盖旧快照。

## 当前状态

本次未执行打包构建（用户未要求）；上一轮安装包仍可运行，但不含本特性。
