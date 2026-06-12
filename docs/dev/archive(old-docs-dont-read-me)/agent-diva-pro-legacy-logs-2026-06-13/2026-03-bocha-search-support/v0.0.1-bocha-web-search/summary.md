# Summary

## Scope

为 Agent Diva 新增博查 Bocha Web Search 搜索引擎支持，并将其设为默认的联网搜索 provider，同时打通核心配置、工具层和 GUI 配置入口。

## Delivered

- 在 `web_search` 工具中新增 `bocha` provider：
  - 接入 `POST https://api.bocha.cn/v1/web-search`，使用 `Authorization: Bearer {API_KEY}`。
  - 支持 `query`、`count`、`freshness`、`summary`、`include`、`exclude` 参数。
  - 统一输出格式：标题 + URL + 文本摘要 + 来源站点 +发布时间。
- 扩展搜索 provider 支持矩阵：
  - 核心配置与校验接受 `brave` / `bocha` / `zhipu` 三种 provider。
  - `bocha` / `zhipu` 的 `max_results` 上限为 50，`brave` 为 10。
- 将默认搜索引擎切换为博查：
  - 核心配置 `default_search_provider()`、运行时 `NetworkToolConfig`、迁移模块默认值全部统一为 `bocha`。
  - GUI 初始 `toolsConfig.web.search.provider` 默认值调整为 `bocha`。
- GUI Network 设置页增强：
  - Provider 下拉支持 `Bocha` / `Brave` / `Zhipu` 三项，并与后端 provider 名称一致。
  - 不同 provider 下 `max_results` 上限与校验逻辑统一：Bocha/Zhipu 为 50，Brave 为 10。
  - 针对 Bocha 的 API Key 文案与占位符文案。
  - 在 Bocha 场景下新增“获取 Bocha API Key 指引”链接，指向提供的飞书文档。

