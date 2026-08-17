# v0.2.0 退役通道后端拆除 — 验证

## 门禁命令与结果（按序）

| # | 命令 | 结果 |
|---|------|------|
| 1 | `cargo fmt`（channels crate）→ `just fmt-check` | ✅ 绿 |
| 2 | `cargo clippy -p agent-diva-channels -- -D warnings`（默认 feature，退役代码不编译） | ✅ 绿 |
| 3 | `cargo test -p agent-diva-channels`（默认 feature） | ✅ 80/80（lib，含新增防回退测试） |
| 4 | 可恢复性证明：`cargo test -p agent-diva-channels --features channel-slack,channel-whatsapp,channel-matrix,channel-irc,channel-mattermost,channel-nextcloud-talk` | ✅ lib 133/133 + qq 集成 6/6 + whatsapp 集成 2/2 |
| 5 | `just check`（workspace clippy `-D warnings`，含 manager/cli） | ✅ 绿 |
| 6 | `cargo test --workspace --exclude agent-diva-cli --exclude agent-diva-gui` | ✅ 绿（磁盘清理后重跑通过） |
| 7 | GUI | 无需重跑（本次无前端改动；GUI 下架已在 v0.1.1 验收） |

## 环境事故与处理（重要背景）

- 首次执行门禁 6 时遭遇 rustc `STATUS_ACCESS_VIOLATION` 与
  `LNK1201/LNK1140`（PDB 写入失败）。排查确认：**C 盘 954 GB 已 100% 写满**，
  主因是 `agent-diva/target/debug` 累积 381 GB（其中 PDB 调试符号约 99 GB）。
- 经用户确认后外科清理 target/debug 下全部 `.pdb`（保留其余编译缓存），
  释放约 100 GB；重跑门禁 6 通过。与本次代码变更无关（channels crate 在
  两种 feature 配置下均独立全绿）。
- 注：`target/debug` 清理 PDB 后增量编译会重建符号，后续构建时间略增，
  无正确性影响。

## 防回退测试

`configured_channel_names_excludes_retired_channels_by_default`（默认 feature
生效）：6 个退役通道全部 `enabled=true` 且必填字段齐全时，
`configured_channel_names` 不收录它们；telegram 齐全作为阳性对照收录。
各退役断言带 `#[cfg(not(feature = "channel-*"))]`，feature 恢复后测试仍可编译。

## 下游不回归证据

- `agent-diva-manager` / `agent-diva-cli` 未改动；`just check` 对其 clippy 全绿。
- `apply_channel_update` / `impl_channel_toggle!` 仅操作保留的 core 配置结构。
- `task_runtime.rs` 经 `configured_channel_names` 自动剔除退役通道。
