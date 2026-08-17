# Verification — retire unverified channels from GUI

## 已执行门禁（2026-08-18）

| 命令 | 结果 |
| --- | --- |
| `npx vitest run`（定向：ChannelCard / ChannelCardView / ChannelsSettings） | 16/16 通过 |
| `npm run test`（全量 GUI） | 63 文件 / 451 tests 全部通过 |
| `npm run build`（含 vue-tsc 类型检查） | 构建成功 |

本次无 Rust 变更，workspace Rust 门禁不受影响（manager 网关分支保留自
`cce52a4e`，已在其迭代验证）。

## 关键断言

- 卡片视图：含已启用 `slack` 的原始配置 map → 只渲染 2 张卡，不出现 Slack。
- 仅含退役频道 → 渲染空状态。
- 列表视图侧边栏：不出现 slack，仍出现 telegram/feishu。
- 向导合并保存、email select 布尔规范化保持正确。

## 未执行（环境限制）

- 真实桌面冒烟：需重启 GUI（必要时网关）后人工确认频道页 7 张卡片、
  向导平台列表、列表视图。见 `acceptance.md`。
