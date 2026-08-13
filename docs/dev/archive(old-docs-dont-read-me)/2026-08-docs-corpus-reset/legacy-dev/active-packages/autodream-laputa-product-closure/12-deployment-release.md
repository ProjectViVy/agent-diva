# 部署与发布

## 1. 发布阶段

1. internal：fake provider、自动触发关闭。
2. developer preview：手动 AutoDream、真实 provider 可选、仅 proposal。
3. release candidate：自动触发显式 opt-in、完整 recovery/metrics。
4. stable：默认手动可用；是否默认自动触发由成本与质量数据决定。

任何阶段都不允许自动绕过治理写 Memory。

## 2. 构建门

- `just fmt-check`
- `just check`
- `just test`
- deletion-proof gate
- GUI tests/build
- Tauri check
- Rust 1.80 独立 target gate
- migration backup/rollback/integrity

## 3. 安装与升级 smoke

- 新 profile：首次启动、provider 设置、手动反思、批准、Recall。
- 已有 profile：旧 runs/proposals 可读，新 run 使用新 pipeline。
- 无 provider：主聊天仍可用，Evolution 明确 degraded。
- 损坏 typed store：fail closed 并提供恢复入口。

## 4. 回滚

二进制回滚不得让新 schema 被旧版本误写。需要 minimum reader version 与 downgrade refusal。数据回滚使用 manifest/backup，不恢复 legacy authority。
