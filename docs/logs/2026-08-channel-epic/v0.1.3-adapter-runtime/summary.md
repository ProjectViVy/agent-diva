# CHANNEL-EPIC C2b 完成摘要

## 完成内容

- 在 core 增加 `ChannelId`、封闭 `ChannelCommand`、capability/probe/limits 和 `ChannelHealth`
  运行时合同；它们不建立第二份 wire schema。
- 新增 clean-break `ChannelAdapter`，长生命周期 listener、命令执行、health 和 stop 均不继承
  或包装旧 handler。
- Adapter Registry 提供确定性登记、路由、能力拒绝和运行中注销保护。
- 每 adapter 建立容量 128 的独立 pacing lane；支持 admission deadline、跨 adapter 隔离、
  capability 降级、文本分片、Retry-After、仅安全命令重试及部分投递 receipt。
- listener supervisor 捕获正常退出、错误和 panic，以 250ms 起步、30s 封顶、带 jitter 的
  指数退避重启；stop 可打断 listener、重连等待和 pacing wait。
- fake adapter smoke 已贯通 Fabric ingress、命令路由、pacing 和真实 typed receipt。

## 边界

C2b 没有迁移 Telegram/Discord/Feishu/DingTalk/Email/QQ，没有装配 Manager bootstrap，也没有
建立旧新桥接。真实 adapter 迁移留给 C5，原子生产切换和旧树删除留给 C6。
