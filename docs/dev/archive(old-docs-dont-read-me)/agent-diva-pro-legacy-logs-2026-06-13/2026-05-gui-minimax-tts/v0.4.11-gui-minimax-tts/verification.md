# 验证记录

## 计划验证项

- GUI 配置链路：`ttsProvider=minimax` 与 `ttsVoiceId` 可保存、回读。
- TTS 合成链路：MiniMax 通过 Tauri/Rust WebSocket 命令返回音频数据给前端播放器。
- 回退链路：MiniMax 失败时回退到浏览器播报。

## 已执行

- `pnpm exec vitest run src/features/diva-pet/voice/services/tts-service.test.ts src/features/diva-pet/types.test.ts src/features/diva-pet/services/pet-config.test.ts`
- `cargo test -p agent-diva-gui`
- `just fmt-check`
- `just check`
- `just test`
- `pnpm build`

## 结果

- 通过：前端受影响测试共 23 项通过。
- 通过：`cargo test -p agent-diva-gui` 通过，覆盖新增 Tauri 命令与既有 GUI Rust 测试。
- 失败：`just fmt-check` 在本机 `cargo fmt` 启动阶段发生 Rust 进程 panic，属于环境侧异常，未看到格式差异输出。
- 失败：`just check` 在 `agent-diva-gui` Tauri build-script 阶段发生同类环境侧 panic，未定位到本次 MiniMax 代码级报错。
- 失败：`just test` 暴露仓库既有非本次改动问题：
  - `agent-diva-tools` 引用 `agent_diva_files::FileMetadata` 路径错误；
  - `agent-diva-providers` 的 `ollama` 相关测试导入与类型推断失败。
- 失败：`pnpm build` 暴露既有前端未使用变量问题，集中在 `avatar-runtime-vrm/*` 与 `DesktopPetOverlay.vue`，不属于本次 MiniMax 改动文件。
