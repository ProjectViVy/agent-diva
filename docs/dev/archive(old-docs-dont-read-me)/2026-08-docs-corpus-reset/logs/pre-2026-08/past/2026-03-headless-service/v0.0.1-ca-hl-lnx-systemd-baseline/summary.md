# CA-HL-LNX-SYSTEMD 完整实施摘要

## 迭代信息

- **迭代目录**：`2026-03-headless-service`
- **版本**：`v0.0.1-ca-hl-lnx-systemd-baseline`
- **CA**：`CA-HL-LNX-SYSTEMD`（Linux systemd 服务）
- **范围**：阶段 0～3 完整实施

## 1. 交付物总览

### 阶段 0：基线确认
- 现状清单（本文档前身）已梳理 Headless 包与 CA-CI-MATRIX 接口。

### 阶段 1：systemd unit 模板（WP-HL-LNX-01）
- `contrib/systemd/agent-diva.service`：systemd unit 模板
- `wbs-headless-service-mode.md`：补充模板来源说明与版本对齐说明

### 阶段 2：安装/卸载脚本与包整合（WP-HL-LNX-02）
- `contrib/systemd/install.sh`：安装脚本（需 `sudo`）
- `contrib/systemd/uninstall.sh`：卸载脚本（保留数据目录）
- `scripts/ci/package_headless.py`：采用 `bin/` 结构，Linux 包包含 `systemd/` 子目录
- `wbs-headless-cli-package.md`：补充 Linux systemd 包结构说明
- `headless-bundle-quickstart.md`：补充 systemd 安装/卸载命令

### 阶段 3：验证矩阵与 CI smoke 设计
- `wbs-validation-and-qa.md`：补充 Linux headless 服务测试矩阵（WP-QA-HEADLESS-01 扩展）
- `wbs-ci-cd-and-automation.md`：补充 Linux headless smoke job 设计（CA-CI-SMOKE 预留）

## 2. 关键变更

- **包结构**：二进制统一放入 `bin/`，Linux 包新增 `systemd/agent-diva.service`、`systemd/install.sh`、`systemd/uninstall.sh`
- **manifest**：`bundle-manifest.txt` 新增 `systemd_files` 字段（仅 Linux）
- **安装流程**：解压后执行 `cd systemd && sudo ./install.sh`
- **卸载策略**：默认保留 `/var/lib/agent-diva` 与 `/var/log/agent-diva`
