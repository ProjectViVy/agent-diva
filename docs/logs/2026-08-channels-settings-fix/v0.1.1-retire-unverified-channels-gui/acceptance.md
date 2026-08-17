# Acceptance — retire unverified channels from GUI

前置：重启 GUI（`just start` 或 Tauri dev），网关无需改动。

1. 打开 设置 → 频道，卡片视图应只显示 7 张卡：Telegram、Discord、飞书、
   钉钉、Email、QQ、Neuro-Link；名称与图标正确。
2. 若本地配置中 `slack` / `whatsapp` / `irc` / `mattermost` /
   `nextcloud_talk` / `matrix` 存在（哪怕 enabled=true），界面上不得出现
   这些卡片。
3. 点击「添加频道」：向导平台网格只含上述 7 个平台（matrix 等无凭据字段
   平台本就不进向导）。
4. 切换到列表视图：侧边栏同样不含退役频道；选中任一在维频道可正常编辑保存。
5. 退役频道的后端配置能力保留：`agent-diva` 配置文件中对应段落仍可被网关
   读取/更新（历史性保留，不在本次验收范围）。
