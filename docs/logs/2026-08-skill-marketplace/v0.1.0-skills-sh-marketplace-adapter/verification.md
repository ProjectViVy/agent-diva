# v0.1.0 skills.sh Marketplace Adapter — Verification

## 自动化验证

### Rust（manager）
- `cargo fmt --all -- --check`：通过（FMT_OK）。
- `cargo check -p agent-diva-manager --all-targets`：通过。
- `cargo test -p agent-diva-manager`：
  - lib 单测 103 通过（含新增 `marketplace::tests` 5 例：搜索映射、上游错误
    透出、快照解析、id 三段式校验、路径段白名单；`skill_service::tests`
    新增 2 例：快照安装落盘、拒绝 history/穿越/缺 SKILL.md/非法 slug）。
  - 集成测试 `skill_marketplace_e2e`：1 例通过——以 wiremock 顶替 skills.sh，
    覆盖短查询 400、搜索映射、非法 id 400、下载快照→写入 skill home→
    `/api/skills` 列表回显、重复安装 409。
  - 既有 `autodream_laputa_e2e` 不受影响。
- 说明：本机系统代理会拦截环回，wiremock 用例通过 `NO_PROXY` 豁免环回
  （`allow_loopback_proxy`），与生产环境无关。

### Tauri（src-tauri）
- `cargo check`（`agent-diva-gui/src-tauri`）：通过。

### GUI 前端
- `npx vitest run`：67 文件 / 472 测试全绿（新增 `MarketplaceTab.test.ts`
  4 例：短查询不发请求、搜索排序+已安装禁用、按 id 安装并刷新、失败重试态）。
- `npm run build`（vue-tsc + vite）：通过。

### 工作区门禁
- `just check`（clippy -D warnings）：通过（exit 0）。

## 手动/真机验证（待用户）

- 真实桌面冒烟：启动网关 + GUI，见 `acceptance.md`。挂 TODOLIST
  `SKILL-MARKETPLACE-DESKTOP-SMOKE`。

## 真实上游连通性（本机抽查）

- `GET https://skills.sh/api/search?q=commit` 返回真实 JSON 技能列表。
- `GET https://skills.sh/api/download/juliusbrussee/caveman/caveman-commit`
  返回真实 `files[]` 快照。以上确认所用端点与字段映射为现行有效协议。
