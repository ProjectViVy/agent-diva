# 通道配置教程

本目录包含 Agent Diva 各 IM 通道的详细配置教程。

## 快速导航

### 常用通道（推荐新手）

| 通道 | 配置难度 | 需要公网 IP | 接入方式 | 教程 |
|------|---------|------------|---------|------|
| **Telegram** | ⭐ 简单 | ❌ | Long Polling | [Telegram 配置教程](telegram.md) |
| **飞书** | ⭐⭐ 中等 | ❌ | WebSocket 长连接 | [飞书配置教程](feishu.md) |
| **钉钉** | ⭐⭐ 中等 | ❌ | Stream 模式 | [钉钉配置教程](dingtalk.md) |

### 其他通道

| 通道 | 配置难度 | 需要公网 IP | 接入方式 | 教程 |
|------|---------|------------|---------|------|
| Discord | ⭐⭐ 中等 | ❌ | WebSocket Gateway | [Discord 配置教程](discord.md) |
| WhatsApp | ⭐⭐ 中等 | ❌ | 桥接服务 | [WhatsApp 配置教程](whatsapp.md) |
| Slack | ⭐⭐ 中等 | ❌ | Socket Mode | [Slack 配置教程](slack.md) |
| QQ | ⭐⭐ 中等 | ❌ | QQ 开放平台 API | [QQ 配置教程](qq.md) |
| Email | ⭐⭐ 中等 | ❌ | IMAP/SMTP | [Email 配置教程](email.md) |
| IRC | ⭐⭐ 中等 | ❌ | IRC 协议 | [IRC 配置教程](irc.md) |
| Mattermost | ⭐⭐ 中等 | ❌ | Mattermost API | [Mattermost 配置教程](mattermost.md) |
| Nextcloud Talk | ⭐⭐ 中等 | ❌ | Nextcloud Talk API | [Nextcloud Talk 配置教程](nextcloud-talk.md) |
| Neuro-Link | ⭐ 简单 | ❌ | WebSocket 服务 | [Neuro-Link 配置教程](neuro-link.md) |

## 配置方式

Agent Diva 提供三种配置方式：

### 方式一：桌面终端程序（推荐新手）

- 🖱️ 可视化表单，点选操作
- ✅ 实时状态检测
- 🔄 一键重启服务

### 方式二：CLI 交互式向导

```bash
agent-diva onboard
```

- 分步骤引导配置
- 自动验证凭证
- 生成配置文件

### 方式三：手动编辑配置文件

编辑 `~/.agent-diva/config.json` 文件：

```json
{
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "your-bot-token"
    }
  }
}
```

## 通用配置流程

1. **选择通道** - 根据需求选择合适的 IM 通道
2. **获取凭证** - 按照对应教程在平台端申请凭证
3. **填写配置** - 在 GUI 或配置文件中填写凭证
4. **测试连接** - 验证通道连接是否正常
5. **启动服务** - 配置完成后启动 Agent Diva

## 故障排查

如遇到问题，请查看：
- [故障排查指南](troubleshooting.md)
- 项目 GitHub Issues
- 用户指南 `docs/userguide.md`

## 文档更新

- 最后更新：2026-04-04
- 适用版本：Agent Diva v0.2+
