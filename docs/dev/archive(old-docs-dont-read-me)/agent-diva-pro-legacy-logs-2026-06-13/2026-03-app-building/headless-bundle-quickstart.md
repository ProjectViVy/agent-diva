# Agent DiVA Headless Bundle Quickstart

此文件是 `CA-CI-MATRIX` 第一阶段产物中的最小运行说明，供 CI 产出的 Headless 压缩包直接复用。

## Bundle Contents

- `bin/agent-diva` 或 `bin/agent-diva.exe`
- `config/config.example.json`
- `services/README.md`
- `README.md`（本文件）
- `bundle-manifest.txt`

## Minimum Run Path

### Windows

```powershell
.\bin\agent-diva.exe gateway run
```

### macOS / Linux

```bash
chmod +x ./bin/agent-diva
./bin/agent-diva gateway run
```

## Optional: Linux systemd 服务安装

Linux 压缩包中包含 `systemd/` 目录，可安装为系统服务：

```bash
cd systemd && sudo ./install.sh
```

卸载服务（保留数据目录）：

```bash
cd systemd && sudo ./uninstall.sh
```

详见 `docs/app-building/wbs-headless-service-mode.md` 中的 `CA-HL-LNX-SYSTEMD`。

## Optional: macOS launchd 服务安装

macOS 压缩包中包含 `launchd/` 目录，可安装为当前用户的 LaunchAgent（无需 sudo）：

```bash
cd launchd && ./install.sh
```

卸载服务（保留日志目录）：

```bash
cd launchd && ./uninstall.sh
```

详见 `docs/app-building/wbs-headless-service-mode.md` 中的 `CA-HL-MAC-LAUNCHD`。

## Notes

- 这是第一阶段的最小占位 README，只保证 Headless artifact 可以被下载、解压和启动。
- 完整的服务化与安装模板请参考 `docs/app-building/wbs-headless-service-mode.md`。
- 完整的分发包结构、示例配置、CI 工件规则与随包文档，请参考 `docs/app-building/wbs-headless-cli-package.md`。
