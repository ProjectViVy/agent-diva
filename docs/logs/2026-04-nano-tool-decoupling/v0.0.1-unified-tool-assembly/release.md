# Release

本次未执行发布或部署。

原因：

- 这是一次架构层代码解耦与 API/配置重组，当前更适合先在主仓与 `.workspace/agent-diva-nano` 本地联调。
- `agent-diva-nano` 已改为 path 依赖主仓 crate，后续若要发布 crates.io，需要单独决定是否恢复版本依赖或引入发布期替换脚本。
