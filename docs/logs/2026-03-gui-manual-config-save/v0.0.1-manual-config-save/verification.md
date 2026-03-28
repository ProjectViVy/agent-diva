# Verification

## Commands

1. `just fmt-check`
   - 结果：通过。

2. `just check`
   - 结果：通过。
   - 备注：存在工作区既有告警，包括 clippy 的 MSRV 提示，以及 `imap-proto v0.10.2` 的 future incompatibility 提示。

3. `just test`
   - 第一次结果：失败。
   - 现象：Windows 下 `target\debug\agent-diva.exe` 与相关测试产物被占用，出现 `LNK1104` 和 access denied。
   - 处理：结束残留的 `cargo test --all` 与 `target\debug\agent-diva.exe` 进程后重试。
   - 第二次结果：在 20 分钟超时窗口内未完成，未观测到本次 GUI 改动直接导致的新错误输出。

4. `npm run build`（工作目录：`agent-diva-gui`）
   - 结果：通过。
   - 作用：完成 GUI 构建级冒烟，确认 Vue + Vite 构建链路可正常通过。

5. `cargo build --release -p agent-diva-cli -p agent-diva-service`
   - 结果：通过。
   - 作用：为 GUI 正式安装包准备嵌入式 CLI / service 二进制。

6. `npm run bundle:prepare`（工作目录：`agent-diva-gui`）
   - 结果：通过。
   - 作用：将 `agent-diva.exe` 与 `agent-diva-service.exe` 分发资源准备到 `src-tauri/resources/`。

7. `npm run tauri build`（工作目录：`agent-diva-gui`）
   - 结果：通过。
   - 产物：
     - `target/release/bundle/nsis/Agent Diva_0.4.1_x64-setup.exe`
     - `target/release/bundle/msi/Agent Diva_0.4.1_x64_en-US.msi`

## Manual/Behavior Checks

- 通过代码路径确认：
  - 供应商设置改为仅更新本地草稿，点击保存后才调用配置保存入口。
  - 频道设置移除了自动 `update_channel` 调用，切换频道不再隐式保存。
  - 网络设置移除了深度 watch 自动保存逻辑，保存前仍会执行本地 sanitize/clamp。
- 本轮未在桌面环境中完成人工点击级 GUI 交互验收，后续可在真实窗口中补充一次“编辑不自动保存，点击按钮才保存”的手工走查。
