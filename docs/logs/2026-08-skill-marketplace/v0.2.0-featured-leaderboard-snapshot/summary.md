# v0.2.0 Featured Leaderboard Snapshot — Summary

## 背景

技能市场标签页只支持关键词搜索（skills.sh API 要求查询至少 2 字符，且无
公开的无认证榜单端点）。用户希望无搜索时展示精选推荐（Top 100 榜单）。

官方榜单 API `GET /api/v1/skills`（view=all-time|trending|hot，分页）存在，
但要求 Vercel OIDC Bearer token（仅在启用 OIDC Federation 的 Vercel Functions
内可获取，短期有效），本地桌面网关无法长期自动使用。

## 决策：离线快照方案

用户拍板：自己写脚本抓取榜单、保存为静态 YAML，token 由我们自行保管；
网关运行时零外部依赖。

## 变更

- **新增** `agent-diva-manager/scripts/fetch_marketplace_featured.py`（纯标准库）：
  - 有 token（`AGENT_DIVA_SKILLS_MARKETPLACE_TOKEN` / `--token` / `--token-file`）
    走官方 `/api/v1/skills?view=&page=&per_page=`，分页取满；
  - 无 token 降级抓取 skills.sh 首页服务端渲染 Leaderboard HTML
    （href + `aria-label="Weekly installs: ..."` sparkline 求和）；
  - 输出 `agent-diva-manager/data/marketplace_featured.yaml`
    （generated_at/source/metric/skills，单引号安全转义）。
- **新增** 快照 `agent-diva-manager/data/marketplace_featured.yaml`：
  首次生成为 web:leaderboard 模式 Top 100（真实数据，周安装量和排序）。
- **manager**：`marketplace.rs` 增加 `FeaturedSnapshot` + `featured_snapshot()`
  （`include_str!` 内嵌 + serde_yaml 解析）；新增
  `GET /api/skills/marketplace/featured` 路由（注册在 `:slug` 通配之前），
  返回 `{status, skills, total, generated_at, source, metric}`。
- **GUI**：新增 Tauri 命令 `featured_marketplace_skills`；`desktop.ts`
  增加 `featuredMarketplaceSkills()`；`MarketplaceTab.vue` 挂载时加载精选，
  空搜索态展示 Top 100（按 installs 排序）+ 快照日期标注；i18n 中英新键
  `marketplaceFeaturedTitle` / `marketplaceFeaturedSnapshot`。

## 影响范围

仅技能市场展示路径；搜索 / 安装 / 安全守卫行为不变。快照随构建内嵌，
刷新需重跑脚本并重新编译（手动节律，符合短期 OIDC token 的现实约束）。
