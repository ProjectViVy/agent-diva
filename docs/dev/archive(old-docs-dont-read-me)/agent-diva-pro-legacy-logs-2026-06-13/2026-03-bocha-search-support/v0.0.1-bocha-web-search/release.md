# Release

## Method

本迭代聚焦功能与配置层开发 + 本地验证，本身不直接执行线上发布，仅给出推荐的发布路径：

1. 在合适的分支上合并本次改动，并通过 CI（`just ci`）。
2. 按既有流程构建并分发更新后的 GUI/Tauri 应用。
3. 更新默认配置/示例说明，标注：
   - 默认搜索 provider 已切换为 Bocha。
   - 需要在 `tools.web.search.api_key` 或 `BOCHA_API_KEY` 中配置 Bocha API Key。

## Suggested Rollout

1. **后端组件**（gateway / manager / agent / channels）：
   - 按环境逐步滚动更新，优先验证内部环境的联网搜索行为。
   - 针对已有依赖 Brave/Zhipu 的配置，保持原 provider 不变（本次默认切换只影响新配置 / 默认配置）。
2. **GUI 客户端**：
   - 发布新版本 GUI，内建默认 provider 为 Bocha。
   - 在发布说明中突出 Bocha 作为新默认搜索引擎，以及获取 API Key 的指引链接。
3. **回滚策略**：
   - 若 Bocha 出现服务质量或额度问题，可通过配置将 `tools.web.search.provider` 回退为 `brave` 或 `zhipu`，无需回滚代码。
   - GUI 侧依然允许用户在设置页切换 provider，回滚可通过配置和 UI 操作完成。***
