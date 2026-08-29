# CHANNEL-EPIC C1 发布说明

## 发布方式

本批是隔离 worktree 中的合同与验证交付，工作树为
`C:\Users\Administrator\Desktop\morediva\agent-diva-channel-epic`，分支为
`feat/channel-epic`。C1 提交不直接合入 `dev`，不推送远端，不启动 Gateway，不迁移 GUI 实时链路，
也不删除旧产品树；保持 C0 冻结的 C6 原子合并策略。

## 运行时影响

新增的是公共 typed contract、schema、fixture、TCK 和容量基准。`agent-diva-core::channel` 在
C1 仅作为数据合同模块公开，不装配 listener、queue、adapter 或 HTTP/WS 路由，因此当前产品
行为不发生切换。

## 回滚

C1 未改变现有运行时路由。若合同批次需要撤回，可撤回 C1 聚焦提交；不得借此恢复已明确退休的
兼容层或把旧协议重新作为第二真相源。C2 开始前应先审阅 schema/TCK 的变更，再继续使用同一
隔离分支。
