# Acceptance: pet → mate 全面改名

## 自动化验收（已通过）

1. `just fmt-check && just check`：通过。
2. `cargo test -p agent-diva-core --lib config::`：70 通过，含旧 `"pet"`
   配置节迁移用例。
3. `cargo test --workspace --exclude agent-diva-cli`：全绿
   （CLI 仅 6 个预存在 wiremock 502 用例）。
4. `cd agent-diva-gui && npm test`：487/487 通过（含 localStorage 迁移用例）。
5. `npx vue-tsc --noEmit` 与 `npm run build`（vite 多页构建含
   `desktop-mate.html` / `embedded-mate.html`）：通过。

## 人工桌面冒烟步骤

1. 启动 GUI（`cd agent-diva-gui && pnpm tauri dev` 或运行已构建的
   `agent-diva-gui.exe`）。
2. 确认侧边栏导航项显示"伙伴"（原"宠物"），点击进入伙伴视图
   （原 DivaPetView，现 DivaMateView），VRM 模型正常渲染。
3. 打开 设置 → 伙伴（原 宠物）页：
   - 标题与"启用伙伴"开关文案正确；
   - 关闭再打开开关，入口显隐正常。
4. 弹出"桌面伙伴"窗口（原桌面宠物弹窗）：窗口标题
   `Diva Desktop Mate`，渲染、暂停/恢复、关闭回主窗口正常。
5. 配置持久化验证：
   - 已有旧配置的用户：启动后原语音/模型/外观设置仍在；
     `~/.agent-diva/config.json` 保存后出现 `"mate"` 节（原 `"pet"`）；
   - 浏览器/GUI DevTools 中 localStorage 出现 `agent-diva-mate-config`。
6. 中英文切换，所有原桌宠相关文案显示为 伙伴 / Mate。

## 冒烟状态

构建级冒烟（vitest + vue-tsc + vite build + Tauri cargo check）已通过；
桌面级人工冒烟按仓库惯例挂 `PET-TO-MATE-DESKTOP-SMOKE` 待用户执行。
