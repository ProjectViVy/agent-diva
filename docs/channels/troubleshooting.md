# 通道配置故障排查指南

本文档汇总了通道配置过程中的常见问题和解决方案。

## 通用问题

### Q1: 配置后通道不工作

**检查清单**：
1. 确认通道已启用（`enabled: true`）
2. 检查凭证是否正确（Token、App ID、Secret 等）
3. 查看日志输出是否有错误信息
4. 确认网络连接正常

**解决步骤**：
```bash
# 查看详细日志
RUST_LOG=debug agent-diva gateway

# 检查配置状态
agent-diva status
```

### Q2: 多个通道同时启用会冲突吗？

不会。每个通道的会话是独立的，互不干扰。你可以同时启用多个通道。

### Q3: 如何查看通道状态？

```bash
# CLI 命令
agent-diva channels status

# 或查看日志输出
```

## 各通道特定问题

### Telegram

| 问题 | 解决方案 |
|------|---------|
| 无法连接 | 检查代理设置，确认 `proxy` 配置正确 |
| 机器人不回复 | 检查 Token 是否正确，确认已点击 Start |
| 群聊收不到消息 | 通过 BotFather `/setprivacy` 关闭隐私模式 |

### 飞书

| 问题 | 解决方案 |
|------|---------|
| 搜索不到机器人 | 确认应用已发布且在可用范围内 |
| 消息收不到 | 检查是否选择了「长连接模式」 |
| 权限不足 | 确认权限已开通并重新发布应用 |

### 钉钉

| 问题 | 解决方案 |
|------|---------|
| 收不到消息 | 确认选择了 Stream 模式（不是 HTTP 模式） |
| Stream 连接失败 | 检查 Client ID 和 Client Secret |
| 应用不可见 | 检查应用是否已发布 |

## 日志分析

### 常见错误信息

**凭证错误**：
```
ERROR Invalid credentials: [具体错误]
```
→ 检查 Token、App ID、Secret 等凭证是否正确

**连接错误**：
```
ERROR Connection failed: [具体错误]
```
→ 检查网络连接，确认代理设置（如需要）

**权限错误**：
```
ERROR Permission denied: [具体错误]
```
→ 在平台端检查应用权限是否已开通

## 获取帮助

如果以上方法无法解决问题：

1. **查看完整日志**: `RUST_LOG=trace agent-diva gateway`
2. **搜索 Issues**: GitHub Issues 中搜索类似问题
3. **提交 Issue**: 提供详细日志和复现步骤

## 相关文档

- [通道配置总览](README.md)
- [用户指南](../userguide.md)
