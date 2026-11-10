# 待办

- [ ] 修正桌面宿主上游格式检查。对 `src-tauri/src/bridge.rs` 的测试调用应用 rustfmt，确保 `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 通过。
- [ ] 处理前端测试依赖的中等严重性审计报告。复核 `@vue/test-utils` 到 `minimatch` 的依赖链，更新锁文件并重新运行前端测试和 `npm audit`。
- [ ] 使用与当前桌面协议匹配的 Windows x64 核心产物完成原生验收。运行真实核心桥接用例、首次启动和 NSIS 安装，核对标准用户安装、125% 和 150% 缩放下的 WebView2 布局、隐藏到托盘后的 Recap 与唤醒行为。详见 [README 验证记录](docs/logs/2026-11-readme-migration/v0.0.1-bilingual-readme/verification.md)。
