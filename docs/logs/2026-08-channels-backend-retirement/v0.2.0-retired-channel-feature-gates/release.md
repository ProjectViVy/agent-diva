# v0.2.0 退役通道后端拆除 — 发布

## 发布方式

随 `agent-diva` 主二进制常规发布，无独立发布物：

- 默认编译（`default = []`）即退役态：6 个退役通道的适配器与注入逻辑
  不参与编译，`slack-morphism` 依赖不拉取。
- 无需任何配置迁移：core 配置 schema 保留退役通道字段，旧配置中的
  退役通道条目静默失效（不再路由、不再启动）。

## 按需恢复

个别部署若需要某个退役通道，重新编译即可：

```bash
cargo build --release -p agent-diva-channels --features channel-slack
# 或同时恢复多个
cargo build --release --features channel-slack,channel-whatsapp,channel-matrix,channel-irc,channel-mattermost,channel-nextcloud-talk
```

## 回滚

`git revert` 两个提交（`refactor(channels)` + `docs(logs)`）即可回到
退役前全量编译状态；无数据/配置破坏，无需回滚迁移。

## 风险说明

- 退役通道在 `just check`/CI 默认矩阵中不再被编译，潜在腐化不会被
  常规门禁发现；恢复时需先跑全 feature 编译验证。
