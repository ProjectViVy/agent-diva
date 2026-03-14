# Summary

- 修复 GUI provider 列表中的 `default_api_base` 映射错误。
- 之前 Tauri 将 provider 当前配置覆盖的 `api_base` 当成了 provider 元数据默认地址，导致 GUI 打开时会出现 `deepseek + https://api.xiaomimimo.com/v1` 这样的错误默认展示。
- 现在 `ProviderView` 明确区分 `default_api_base` 与 `api_base`，GUI 默认展示使用 provider 元数据默认地址，不再把运行时覆盖地址误当成默认地址。
