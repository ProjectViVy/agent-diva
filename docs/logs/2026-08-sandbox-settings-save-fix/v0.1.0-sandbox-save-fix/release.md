# release：沙箱设置保存修复

## 发布方式

本修复为 GUI 前端变更，随下一次 `agent-diva-gui` 桌面构建发布：

- Windows 安装包：`just package-windows-gui`（NSIS + MSI）。
- CI 桌面构建：`just trigger-build`（GitHub Actions CI，三平台 GUI 构建）。

无配置迁移：`~/.agent-diva/config.json` 的 `sandbox` 段格式（snake_case）未变，
存量配置文件无需处理。后端 Rust 代码未改动，gateway/CLI 无需重新发布即可配合旧版
GUI 工作；但旧版 GUI 的该缺陷只有升级到含本修复的 GUI 构建后才消除。

## 本次未发布的原因

仓库当前处于多 story 开发中（分支 agent-diva-pro 领先 origin），按仓库惯例
仅本地提交、不 push，等待统一发布窗口。
