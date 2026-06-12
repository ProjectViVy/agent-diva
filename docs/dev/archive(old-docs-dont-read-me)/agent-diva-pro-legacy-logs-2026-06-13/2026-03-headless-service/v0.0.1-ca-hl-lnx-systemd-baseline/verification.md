# CA-HL-LNX-SYSTEMD 实施验证记录

## 验证范围

- **CA**：`CA-HL-LNX-SYSTEMD`
- **迭代**：`v0.0.1-ca-hl-lnx-systemd-baseline`
- **验证类型**：文档与脚本完整性、结构一致性

## 1. 阶段 1 验证（systemd unit 模板）

| 检查项 | 预期 | 结果 |
|--------|------|------|
| `contrib/systemd/agent-diva.service` 存在 | 是 | 已创建 |
| 模板内容与 wbs-headless-service-mode.md 示例一致 | 是 | 已对齐 |
| wbs-headless-service-mode.md 指向模板路径 | 是 | 已补充「模板来源说明」与「版本对齐说明」 |

## 2. 阶段 2 验证（安装/卸载脚本与打包）

| 检查项 | 预期 | 结果 |
|--------|------|------|
| `contrib/systemd/install.sh` 存在且可执行 | 是 | 已创建，`chmod +x` |
| `contrib/systemd/uninstall.sh` 存在且可执行 | 是 | 已创建，`chmod +x` |
| install.sh 仅 Linux 执行 | 是 | `uname -s` 检查 |
| uninstall.sh 保留数据目录 | 是 | 不删除 `/var/lib/agent-diva`、`/var/log/agent-diva` |
| package_headless.py 使用 bin/ 结构 | 是 | 二进制放入 `bin/` |
| Linux 包包含 systemd/ 子目录 | 是 | `systemd/agent-diva.service`、`install.sh`、`uninstall.sh` |
| bundle-manifest.txt 含 systemd_files | 是 | Linux 包写入 `systemd_files=...` |
| wbs-headless-service-mode.md WP-HL-LNX-02 引用实际脚本 | 是 | 已更新为对 `contrib/systemd/` 的引用 |
| wbs-headless-cli-package.md 列出 Linux systemd 包 | 是 | 已补充「Linux systemd 包」小节 |
| headless-bundle-quickstart.md 含 systemd 安装说明 | 是 | 已补充「Optional: Linux systemd 服务安装」 |

## 3. 阶段 3 验证（QA 与 CI smoke 设计）

| 检查项 | 预期 | 结果 |
|--------|------|------|
| wbs-validation-and-qa.md 含 Linux headless 测试矩阵 | 是 | 已补充「Linux systemd 服务测试矩阵」 |
| 测试场景覆盖：首次安装、自启、启停、升级、卸载 | 是 | 已列出 5 类场景与观察点 |
| wbs-ci-cd-and-automation.md 含 Linux headless smoke job 设计 | 是 | 已补充「Linux Headless systemd smoke job 设计」 |
| smoke 步骤可映射为 GitHub Actions YAML | 是 | 步骤列表与命令示例已给出 |

## 4. 手工 smoke 路径（待 Linux 环境执行）

以下路径需在 Linux VM 或 CI Linux runner 上执行以完成端到端验证：

1. **最小启动**：`tar -xzf agent-diva-*.tar.gz && ./bin/agent-diva gateway run`
2. **服务安装**：`cd systemd && sudo ./install.sh`
3. **服务状态**：`systemctl status agent-diva` → `active (running)`
4. **日志验证**：`journalctl -u agent-diva -f` 可见网关日志
5. **服务卸载**：`cd systemd && sudo ./uninstall.sh`
6. **清理验证**：`systemctl list-unit-files | grep agent-diva` 无结果，`/usr/bin/agent-diva` 已删除，`/var/lib/agent-diva` 与 `/var/log/agent-diva` 保留

## 5. 结论

- 文档与脚本已按计划落地，结构一致。
- 手工 smoke 需在具备 systemd 的 Linux 环境中执行，建议在下次 CI 或 Release 验收时补全。
- 后续 `CA-CI-SMOKE` 实现时可直接引用 `wbs-ci-cd-and-automation.md` 中的 smoke job 设计。
