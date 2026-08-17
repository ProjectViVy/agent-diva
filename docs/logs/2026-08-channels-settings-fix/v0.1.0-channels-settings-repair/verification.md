# Verification — v0.1.0 channels-settings-repair

## 自动化验证

### GUI（agent-diva-gui，pnpm）

- `npm run test`（vitest）：**63 文件 / 448 tests 全绿**（基线 435 + 新增 13）。
  - `ChannelCard.test.ts` ×5：规范通道渲染显示名/启用态、未知平台回退原始名、
    缺失字段展示、toggle/edit/delete 事件。
  - `ChannelCardView.test.ts` ×4：原始配置 map 归一化为带名称/启用态卡片（含
    neuro-link、matrix）、空状态、toggle 冒泡配置键、status 按名匹配。
  - `ChannelsSettings.test.ts` ×4：**切换非选中卡片也持久化**、向导完成合并既有
    配置、irc 凭据归一化（`channels_str`→`channels`、`'true'`→true）、编辑通道时
    向导 `credentials` 预填。
- `npm run build`（vue-tsc --noEmit + vite build）：通过。

### 网关（agent-diva-manager）

- `cargo test -p agent-diva-manager runtime_control`：通过，含新增
  `apply_channel_update_routes_newly_supported_channels`（四新通道字段写入 +
  enabled 覆盖 + `"neuro-link"` → `neuro_link`）与
  `apply_channel_update_rejects_unknown_channel`。
- `cargo fmt -p agent-diva-manager`、`cargo clippy -p agent-diva-manager
  --all-targets -- -D warnings`、`cargo test -p agent-diva-manager`：全部通过。

### 工作区

- `just fmt-check`、`just check`：通过。
- `cargo test --workspace --exclude agent-diva-cli --exclude agent-diva-gui`：
  全部通过。排除原因：用户正在运行的网关/桌面进程（agent-diva.exe /
  agent-diva-gui.exe）锁定 `target/debug` 二进制，链接阶段"拒绝访问"；
  属环境限制而非代码问题，两个被排除 crate 的 Rust 测试待下次重启后补跑
  （CLI 另有 6 个既有 CLI-WIREMOCK-502-PREEXISTING 失败）。

## 手工冒烟（gui-changes-need-gui-smoke）

待用户/桌面环境执行（见 acceptance.md）：

1. 启动网关与 GUI，打开 设置 → 频道。
2. 卡片视图应显示 13 张卡片，各自有平台名称与图标（matrix 为兜底图标），
   启用状态与配置文件一致。
3. 切换任意非选中卡片的开关 → 配置文件对应通道 `enabled` 更新，刷新后保持。
4. 列表视图选择 slack/irc 等通道 → 显示"通过配置向导编辑"，打开向导字段预填，
   保存后持久化且非向导字段（如 discord allow_from）保留。

## 回归风险说明

- 网关 `update_channel` 为整体反序列化替换（既有行为未变），前端已改为合并保存以保护
  非向导字段；telegram/discord 内联表单保存路径不变。
- `get_channels_handler` 回退路径仅在 runtime 通道失败时触发，正常路径不受影响。
