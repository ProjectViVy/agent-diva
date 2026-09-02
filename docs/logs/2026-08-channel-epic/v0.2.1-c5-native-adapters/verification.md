# Verification

本阶段在 `C:\Users\Administrator\Desktop\morediva\agent-diva-channel-epic` 执行，未使用
真实 token、secret 或外部平台网络。

## 已通过

- `cargo check -p agent-diva-channels`
- `cargo test -p agent-diva-channels --lib`：126 tests passed
- `cargo test -p agent-diva-channels --all-targets`：lib、runtime/shared/characterization TCK
  与 QQ reconnect integration 全部通过（共 126 + 11 + 5 + 5 + 6 项）
- `cargo test -p agent-diva-channels --test channel_adapter_shared_tck -- --nocapture`：5 passed
- `cargo clippy -p agent-diva-channels --lib -- -D warnings`：passed
- `just fmt-check`：passed
- `just check`：passed
- `just test`：最终复跑在既有 `agent-diva-manager::neuro_link::tests::loopback_websocket_handshake_session_and_turn_smoke`
  出现一次 `left: Null, right: 1`；未改动 Manager，随后定向复跑该测试 1 passed。该全量 flake
  与本阶段渠道变更无关，已登记 `MANAGER-NEURO-LINK-LOOPBACK-FLAKE`。
- `git diff --check`：passed

## 仍需独立批次处理

- `cargo clippy -p agent-diva-channels --all-targets -- -D warnings`：历史 legacy channel 测试的
  `field_reassign_with_default`、QQ integration `useless_conversion/collapsible_if` 仍会阻断，
  对应 TODO：`CHANNEL-ALL-TARGETS-CLIPPY-LEGACY-TESTS`。
- `just msrv-probe check -p agent-diva-channels`：缓存 `base64ct v1.8.3` 的 edition/MSRV 与
  Rust 1.80 声明冲突，属于既有 workspace 依赖债务，见 `WORKSPACE-MSRS-1.80-DEPENDENCY-CONFLICTS`。
- C5-V 的每个 capability wire mock、admission busy/cancel、429、attachment transcript 和
  QQ official intents/media/真机 smoke 尚未执行，不能将 manifest 状态改为 `verified`。
