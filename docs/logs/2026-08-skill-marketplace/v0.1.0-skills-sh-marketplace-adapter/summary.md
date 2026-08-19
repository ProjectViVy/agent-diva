# v0.1.0 skills.sh Marketplace Adapter — Summary

## 背景

Settings → 技能 → 「技能市场」标签页此前由前端浏览器直接请求虚构的
`https://skills.sh/api/search` 与 ZIP 下载 URL：接口不存在、字段不匹配、
且 Tauri webview 受 CORS 限制，页面永远停在空态。本次迭代将其接到真实的
skills.sh（Vercel「Agent Skills Directory」，即 `npx skills` CLI 背后的目录服务）。

## 变更内容

### agent-diva-manager（网关适配器）

- 新增 `src/marketplace.rs`：`MarketplaceClient`（reqwest，30s 超时、
  UA `agent-diva/0.5.0`）对接两个真实免认证端点：
  - `GET https://skills.sh/api/search?q=&limit=` →
    `{ skills: [{ id, name, source, installs }] }`（模糊搜索，limit 上限 50）
  - `GET https://skills.sh/api/download/{owner}/{repo}/{slug}` →
    `{ files: [{ path, contents }], hash }`（完整技能文件快照）
  - 支持 `AGENT_DIVA_SKILLS_MARKETPLACE_URL` 环境变量覆盖 base URL。
  - `parse_skill_id` 校验 `owner/repo/slug` 三段式 id；路径段字符白名单
    防注入/穿越。
- `SkillService::install_marketplace_snapshot`：复用 ZIP 上传的全部守卫——
  总大小限制（`validate_skill_zip_size`）、路径规范化、拒绝 `history/`
  伪造、必须含根级 `SKILL.md`、slug 校验，最终走 `SkillHome::install_new`
  （含历史记录）。
- 新增两条路由（`runtime_routes`，注册在 `/api/skills/:slug` 之前）：
  - `GET /api/skills/marketplace/search?q=&limit=`（q < 2 字符 → 400）
  - `POST /api/skills/marketplace/install` body `{ "id": "owner/repo/slug" }`
    （上游失败 → 502 `marketplace_upstream_error`；重复安装 → 409，
    与「市场只新装」语义一致）

### agent-diva-gui（Tauri 桌面端）

- `src-tauri/src/commands.rs`：新增 `search_marketplace_skills` /
  `install_marketplace_skill` 命令，经既有 `memory_request` HTTP 助手
  调用网关，并在 `lib.rs` 注册。
- `src/api/desktop.ts`：删除浏览器直连 skills.sh 的 fetch 实现与虚构类型，
  改为 `invoke()` 包装；`MarketplaceSkillEntry` 收敛为真实字段
  `{ id, name, source, installs }`。
- `MarketplaceTab.vue`：重写为「搜索即结果」体验——输入 ≥2 字符触发
  300ms 防抖搜索，按安装量排序展示卡片（名称 / owner/repo / 安装数），
  已安装技能按 slug 比对禁用安装按钮；移除目录不存在的分类/信任等级/
  排序/分页筛选。新增 i18n 键 `marketplaceSearchPrompt`（zh/en）。

## 影响范围

- 仅影响 Settings 技能市场标签页与 manager 技能路由；已安装技能、
  Evolution 提案、ZIP 上传路径均未改动。
- skills.sh 目录无分类/评分/分页能力，UI 相应简化；搜索需 ≥2 字符
  是上游 API 契约。

## 相关锁与接替

接替 Grok 于 2026-08-19T23:20 声明后因额度耗尽未落地的同范围锁
（见 `LOCK.md` Handoff Notes 2026-08-19T23:37 条目）。
