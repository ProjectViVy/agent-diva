# 验证

## 聚焦验证

- `cargo test -p agent-diva-autodream`：通过，35 个测试与 doc tests 全部成功。
- `cargo test -p agent-diva-manager autodream --lib`：通过，2 个手动触发链测试成功。
- Manager 成功场景验证：有 session evidence 时运行终态为 completed，并仅创建一个
  Laputa proposal。
- Manager 失败场景验证：无有效输入时运行终态为 failed，返回
  `failure_code=input_unavailable`。
- Worker 验证：成功、超时、取消、输入缺失分别写入预期终态和稳定失败码；失败不推进
  checkpoint。

## GUI 验证

- `pnpm test -- EvolutionView.test.ts locales/evolution.test.ts`：通过，19 项。
- `pnpm build`：通过；保留既有 Vite 大 chunk 警告。
- `cargo check --manifest-path agent-diva-gui/src-tauri/Cargo.toml`：通过。

## 工作区总门

- `just fmt-check`：通过。
- `just check`：通过。
- `just test`：首次执行因正在运行的桌面程序锁定
  `target/debug/agent-diva.exe` 而中止，错误为 Windows `os error 5`，不是测试断言失败。
- 使用独立 `CARGO_TARGET_DIR=target-e0-validation` 复跑 `cargo test --all`：全部已执行
  crate 在到达 GUI lib test 前通过；GUI lib test 链接阶段被 MSVC `LNK1140`
  （程序数据库限制）阻断。单独复跑得到相同链接器错误。GUI 的 Rust
  `cargo check`、前端测试和生产 build 均通过。

该环境阻断不会被记作完整测试通过，也不冒充真实桌面验收；E7 发布门必须在释放桌面
二进制锁、采用可承载 GUI test 链接的构建环境后重新执行完整 `just test`。

## 人工验证

按项目决策，本阶段不要求用户进行真实桌面测试。最终人工验证统一延后至 E0–E7
自动化闭环和发布门完成后的 G2D+。
