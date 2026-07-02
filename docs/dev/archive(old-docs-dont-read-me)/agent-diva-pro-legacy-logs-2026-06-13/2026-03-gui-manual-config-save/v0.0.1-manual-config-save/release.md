# Release

## Release Type

- Windows GUI 正式版打包

## Version

- `v0.4.1`

## Release Method

1. 构建 release CLI / service 二进制：
   - `cargo build --release -p agent-diva-cli -p agent-diva-service`
2. 准备 Tauri 安装包资源：
   - `npm run bundle:prepare`
3. 生成桌面端正式安装包：
   - `npm run tauri build`

## Output

- `target/release/bundle/nsis/Agent Diva_0.4.1_x64-setup.exe`
- `target/release/bundle/msi/Agent Diva_0.4.1_x64_en-US.msi`

## Notes

- 本轮确认安装产物文件名已经携带 `0.4.1` 版本号，可作为 `v0.4.1` 正式版交付物。
