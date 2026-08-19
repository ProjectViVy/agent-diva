# Acceptance — v0.1.2 channel editor GUI

重启 GUI 后打开 **设置 → 频道**。

## A. 卡片编辑不再空白

- [ ] 默认卡片模式，点飞书卡片的编辑（铅笔）。
- [ ] 仍停留在卡片页，弹出「编辑通道」向导。
- [ ] 不出现平台选择网格；App ID / App Secret 等字段可见，已有值被预填。
- [ ] 点 Email 卡片编辑，IMAP/SMTP 字段可见；高级设置默认折叠。

## B. 编辑布局

- [ ] 向导表单可滚动，高级组折叠后主表单不挤成一团。
- [ ] 切到列表视图：左侧窄栏（约 220px）列出可见通道，右侧有 padding 的详情表单。
- [ ] 每个可见通道（telegram / discord / 飞书 / 钉钉 / Email / QQ / Neuro-Link）
      选中后都有字段，而不是空提示。

## C. 不再要求改 YAML

- [ ] 页面上不再出现「请直接编辑配置文件」或「此通道暂无内联表单」。
- [ ] 改飞书 App ID 并完成向导，刷新后值仍在。
- [ ] 编辑 Discord 高级组的允许用户 ID，保存后既有 token 不被清空。
- [ ] 添加通道仍先选平台，再填表单。

## D. 退役通道仍隐藏

- [ ] 卡片、列表侧栏、向导平台网格都不出现 slack / whatsapp / matrix / irc /
      mattermost / nextcloud_talk。
