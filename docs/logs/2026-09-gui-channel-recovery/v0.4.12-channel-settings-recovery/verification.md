# Channel settings recovery verification

日期：2026-09-04

## 最终自动化门禁

最终后端集成 HEAD `53b3dded` 的顺序 QA：

- `just fmt-check`：通过。
- `just check`：通过；仅保留 `imap-proto v0.10.2` future-incompat warning。
- `just test`：通过，`2288 passed / 0 failed / 36 ignored`；Manager websocket、GUI Mate、
  Laputa flake 均未复现。
- `just channel-clean-break-check`：通过。
- `python scripts/ci/check_gui_dependency_policy.py`：通过。
- `corepack pnpm install --frozen-lockfile --offline`：通过，lockfile 未改变。
- `corepack pnpm test -- --maxWorkers=2`：77 个文件、562 个测试通过。
- `corepack pnpm build`：通过；保留 Vite 大于 500 kB 的 chunk warning，最大约 5.78 MB。
- `cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml --lib -- --test-threads=1`：
  61 个测试通过。
- Rust 1.80 独立 target 的 `cargo +1.80.0 check -p agent-diva-channels`：通过；未再复现
  `getrandom` Edition2024 blocker。
- CLI `--help`：第一次只完成编译、未取得最终 help 输出；随后重跑通过并输出完整帮助。

浏览器发现修复后的 HEAD `46402c92` 另行执行：

- 频道刷新 i18n 修复后再次运行 GUI 全量：77 个文件、562 个测试通过；build 通过。
- `ChatView` Lucide `X` 修复后运行两个 ChatView 测试文件：24 个测试通过；build 通过。
- 通过 pnpm 获取的 `agent-browser 0.36.0` 在 Node 24.14.1 下打开本地 Vite；主页和
  设置入口可交互，频道页标题与“刷新”文案正确，交互 snapshot 可读取，页面无
  `vite-error-overlay`/`alertdialog`。最终 console 不再出现未知组件 `X` 或缺失
  `topbar.refresh` 翻译告警。
- 纯浏览器没有 Tauri Host，因此频道页显示 `invoke` 不可用的可重试错误态；这不是桌面
  command 失败，真实 bridge 已由上述 61 个 Tauri 测试覆盖。

## 审查门禁

独立后端审查在首轮发现 probe caller-drop、Email drain、supervisor stop handoff、runtime
transition、Manager 跨层 mutation 及其他配置 writer 交错等 P1。经 `dbe70847`、
`bd62a095`、`5fc842ad`、`53b3dded` 逐项修复和 future-abort/race 回归后，最终结论为
`APPROVE (P0=0, P1=0)`。依赖审查也为 APPROVE。

## 如实保留的非最终运行结果

- 早期全工作区验证在链接 GUI 时因 C 盘 `os error 112` 失败，测试尚未开始；确认无相关
  进程后，仅对隔离 worktree 执行 `cargo clean`，先后清理 35.7 GiB 与 25.7 GiB 生成缓存。
  最终验证全部使用 D 盘隔离 target 并通过。
- 两次默认 worker GUI 全量运行分别出现 1 个和 3 个 Mate 超时，另有一次嵌入式 health
  启动时序失败；`--maxWorkers=2` 的最终全量稳定通过。复发债务已重开到 `TODOLIST.md`。
- 本轮未重复宣称联网 audit：修复时 audit 曾返回零漏洞，后续 registry 请求挂起的事实记录
  在 v0.4.11；持续 CI job 仍 fail-closed。
- 未提供或使用任何真实频道凭据，未运行 credential-gated live harness。
