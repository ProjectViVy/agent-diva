# v0.2.1 发布说明

## 发布方式

随下一次桌面端重建发布生效(网关与 GUI 需同时更新):

1. 重建:`cargo build --release`(或既有 `scripts/package-windows-gui.ps1` 打包流水线)。
2. 重启网关与 GUI。

## 兼容性

- 网关新字段 `evolution_managed` 对旧 GUI 透明(被忽略)。
- 新 GUI 连旧网关时 `#[serde(default)]` 使字段缺省为 `false`,此时所有 Home 技能
  均显示删除按钮(退化为可删,不出现错误提示),可接受。
- 无配置迁移、无磁盘格式变更。

## 回滚

还原本提交即可;技能存储格式未变,无数据回滚需求。
