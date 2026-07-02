# 验证记录

## 已执行

- `pnpm -C agent-diva-gui test`
  - 结果：通过，20 个测试文件、265 个测试。
- `pnpm -C agent-diva-gui build`
  - 结果：通过，`vue-tsc --noEmit` 与 Vite build 完成。
- `cargo test -p agent-diva-gui pet_vrm_model_tests --no-default-features`
  - 结果：通过，4 个 VRM 模型命令相关单测。
- `cargo fmt --all`
  - 结果：已格式化。
- `just fmt-check`
  - 结果：通过。
- `just check`
  - 结果：通过。
- `just test`
  - 结果：失败；失败点在既有 `agent-diva-providers` Ollama 测试，`agent_diva_providers::ollama` 模块不可用，非本次 VRM 改动引入。
- GUI 页面 smoke
  - 命令：`pnpm -C agent-diva-gui exec vite --host 127.0.0.1 --port 1421`
  - 结果：启动成功，`http://127.0.0.1:1421` 返回 HTTP 200。

## 未覆盖

- 未执行完整 Tauri 桌面窗口手工操作；需要在桌面应用中实际导入 `.vrm` 并观察角色切换。
