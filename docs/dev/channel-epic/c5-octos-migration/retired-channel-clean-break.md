# 退休频道 Clean Break inventory

C5-P2 不为以下频道新增适配器或迁移能力：Slack、WhatsApp、Matrix、IRC、Mattermost、Nextcloud
Talk 及其它已从六个现役频道范围移出的历史通道。

后续 C6 只需完成：

- 从 production Manager、feature flags、配置别名和构建脚本删除运行时入口。
- 使用 clean-break `rg` gate 证明旧源码、旧 DTO、旧 bus、wrapper/shim 未重新进入产品路径。
- 在删除前记录是否仍有离线迁移数据或用户配置需要提示；不在 C5 中恢复实现。

若扫描 agent 在 Octos 发现退休频道仍有有价值的协议模式，只能作为研究链接写入研究文档，
不得添加到 C5 capability matrix，也不得创建新的 product dependency。
