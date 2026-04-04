# Telegram 配置教程

> Telegram 是最容易配置的 IM 通道，只需一个 Bot Token 即可，无需公网 IP。

## 平台概览

- **接入方式**: Long Polling
- **需要公网 IP**: ❌ 不需要
- **配置难度**: ⭐ 最简单

## 前置条件

- 一个 Telegram 账号（手机号注册即可）
- 能访问 Telegram（大陆环境需要代理）

## 平台端申请步骤

### 第一步：打开 BotFather

在 Telegram 搜索栏中搜索 `@BotFather`，点击进入对话。BotFather 是 Telegram 官方的机器人管理工具。

### 第二步：创建新机器人

1. 向 BotFather 发送 `/newbot` 命令
2. BotFather 会要求你为机器人设置一个**显示名称**（name），例如 `My Agent Diva Bot`
3. 接着要求设置一个**用户名**（username），必须以 `bot` 结尾，例如 `my_agent_diva_bot`

### 第三步：获取 Bot Token

创建完成后，BotFather 会返回一条消息，其中包含你的 **Bot Token**，格式类似：

```
123456789:ABCDefGH-ijklMNOPqrstUVWxyz1234567
```

⚠️ **重要**：妥善保管你的 Bot Token，不要泄露给他人。

### 第四步：（可选）设置机器人头像和描述

- 发送 `/setuserpic` — 设置机器人头像
- 发送 `/setdescription` — 设置机器人简介
- 发送 `/setcommands` — 设置命令菜单（如 `/start` - 开始对话）

### 第五步：（可选）配置机器人隐私

默认情况下，机器人在群聊中只能收到 `/command` 格式的消息和 @机器人的消息。如果需要接收群聊中的所有消息：

1. 向 BotFather 发送 `/setprivacy`
2. 选择你的机器人
3. 选择 `Disable`（关闭隐私模式）

## Agent Diva 配置

### 方式一：GUI 配置（推荐）

1. 打开通道配置页面
2. 点击"添加通道"选择 Telegram
3. 在 Bot Token 输入框中粘贴 Token
4. 点击"下一步"测试连接
5. 测试通过后完成配置

### 方式二：CLI 配置

```bash
agent-diva onboard
```

按提示选择 Telegram 并输入 Bot Token。

### 方式三：手动编辑配置文件

编辑 `~/.agent-diva/config.json`：

```json
{
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "你的 Bot Token"
    }
  }
}
```

### 可选配置（代理）

大陆环境可能需要配置代理：

```json
{
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "你的 Bot Token",
      "proxy": "http://127.0.0.1:7890"
    }
  }
}
```

## 验证与测试

1. 启动 Agent Diva
2. 在 Telegram 中搜索你的机器人用户名
3. 点击 **Start** 按钮（或发送 `/start`）
4. 发送一条测试消息，等待机器人回复

## 常见问题

| 问题 | 解决方案 |
|------|---------|
| 无法连接 Telegram | 检查代理设置，确认代理地址正确 |
| 机器人不回复 | 检查日志输出，确认 Token 正确 |
| 群聊中收不到消息 | 检查是否需要关闭隐私模式（通过 BotFather `/setprivacy`） |
| 提示"配对码错误" | 确认配置正确 |

## 相关文档

- [用户指南](../userguide.md)
- [故障排查](troubleshooting.md)
