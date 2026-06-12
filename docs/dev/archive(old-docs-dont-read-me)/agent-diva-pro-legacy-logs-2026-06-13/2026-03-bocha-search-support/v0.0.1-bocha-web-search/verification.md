# Verification

## Commands

- `just fmt-check`
- `just check`
- `just test`
- `npm run build` （在 `agent-diva-gui` 目录）
- `npm run dev -- --host 127.0.0.1 --port 4173`（GUI 本地开发服务，用于手动 smoke）

## Results

- `fmt-check` 通过，工作区 `rustfmt` 无格式错误。
- `check` 通过，`clippy` 在所有 crate 上无新的告警（仅保留已有 `imap-proto` future incompat 警告）。
- `test` 通过，所有工作区 crate 的单测和 doc-test 通过，包括：
  - `agent-diva-tools` 新增的 `web_search_max_results_limit` 测试。
  - 配置校验相关测试（包含默认配置、Bocha max_results 验证）。
- GUI 前端 `npm run build` 成功，Vite 构建产物生成正常，仅有体积提示告警。

## GUI Smoke（由用户在本地浏览器完成）

- 能成功打开 `http://127.0.0.1:4173/` 并进入设置页。
- 在 Network / 网络工具设置中：
  - 默认搜索 provider 为 Bocha。
  - Provider 下拉包含 Bocha / Brave / Zhipu。
  - 选择 Bocha 时，API Key 标签为 Bocha 对应文案。
  - Bocha 场景下可以看到“获取 Bocha API Key 指引”的超链接，跳转至飞书文档：
    - `https://aq6ky2b8nql.feishu.cn/wiki/HmtOw1z6vik14Fkdu5uc9VaInBb`
  - 手动尝试 `max_results` 时，Bocha/Zhipu 上限为 50，Brave 上限为 10（通过输入大数并观察控件回填验证）。

## Notes

- 本次变更未引入新的网络依赖开关，Bocha 仅在配置中启用且提供 API Key 后才会真正调用外部接口。
- CI 管线可沿用现有 `just ci` 流程，无需额外脚本调整。***
