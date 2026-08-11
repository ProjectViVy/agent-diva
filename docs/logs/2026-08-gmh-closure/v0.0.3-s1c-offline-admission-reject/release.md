# Release

本 slice 为 agent 内部 admission 门控，无独立发布物、无外部服务依赖。

## 部署方式

随 `agent-diva-agent` 常规 crate 发布即可。`max_actions_per_hour` 默认 100，
正常使用不会触发；运维可通过 JSON security 文件调整。

## 说明

- 无停机、无迁移。
- 默认行为：`max_actions_per_hour` 首次真正生效（用户可感知的限流），
  熔断触发时拒绝入场。day/hour 排队分支待产品决策后另行发布。