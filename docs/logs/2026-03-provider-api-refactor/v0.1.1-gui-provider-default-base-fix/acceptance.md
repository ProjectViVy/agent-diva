# Acceptance

1. 打开 GUI Provider 设置页。
2. 当默认 provider 为 `deepseek` 且配置中存在自定义 `api_base` 覆盖时，provider 默认地址展示不应再直接被覆盖成第三方地址。
3. Rust `cargo check` 与 GUI `npm run build` 必须通过。
