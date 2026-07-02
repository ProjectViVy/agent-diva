# Acceptance

## User Checks

1. **Bocha 作为默认搜索引擎**
   - 启动 CLI/TUI/GUI 任一入口，在未手动改动配置的前提下，确认默认 `tools.web.search.provider` 为 `bocha`。
   - 在 GUI Network 设置页中，打开配置面板，确认 Search Provider 默认选中 Bocha。

2. **搜索能力行为**
   - 在支持工具调用的对话里，触发 `web_search` 工具，输入任意中文或英文查询：
     - 确认返回结果包含标题、URL、摘要，以及来源站点信息。
     - 结果条数不超过配置中的 `max_results`，且在 Bocha 模式下上限为 50。

3. **GUI 设置联动**
   - 在 GUI Network 设置页中：
     - Provider 下拉能在 Bocha / Brave / Zhipu 间切换。
     - 切换为 Bocha 或 Zhipu 时，`max_results` 的允许范围为 1–50；切换为 Brave 时范围为 1–10。
     - Bocha 场景下 API Key 标签显示为 Bocha 对应文案。
     - Bocha 场景下能看到“如何获取 Bocha API Key”链接，点击后在浏览器中打开飞书文档：
       - `https://aq6ky2b8nql.feishu.cn/wiki/HmtOw1z6vik14Fkdu5uc9VaInBb`

4. **兼容性与回退**
   - 对已有使用 Brave/Zhipu 的配置文件进行验证：
     - 保持 provider 不变时，启动和调用行为不受本次改动影响。
     - GUI 中切换 provider 并保存后，配置文件中的 provider 字段与 `max_results` 均按预期更新。

## Acceptance Criteria

- Bocha 搜索在工具层可被正常调用，返回结构符合预期格式。
- 默认配置和 GUI 均将 Bocha 视为首选搜索引擎，但仍兼容 Brave/Zhipu。
- GUI 中 Bocha API Key 的获取指引对终端用户可见且可用。
- 全量 `just fmt-check` / `just check` / `just test` 通过，无新的编译错误或严重告警。***
