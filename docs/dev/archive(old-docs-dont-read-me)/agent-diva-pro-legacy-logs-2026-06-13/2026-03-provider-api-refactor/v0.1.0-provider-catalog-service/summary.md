# Summary

- 引入统一的 `ProviderCatalogService`，把 provider 视图、provider 解析、access 获取、模型目录聚合、自定义 provider/model 变更收敛到 `agent-diva-providers`。
- 扩展配置模型，新增 `providers.custom_providers` 和 `ProviderConfig.custom_models`，同时保留现有固定 provider 槽位兼容。
- manager API 改为返回统一 provider DTO，并补充 provider/provider-model 的 CRUD 与 provider resolve 接口。
- CLI 和 Tauri 去掉重复的 provider 槽位 `match` 逻辑，改为复用统一 service；自定义 OpenAI-compatible provider 发送原始 model id，不再误加前缀。

# Impact

- 影响 crate：`agent-diva-core`、`agent-diva-providers`、`agent-diva-manager`、`agent-diva-cli`、`agent-diva-gui/src-tauri`、`agent-diva-migration`
- 向后兼容：保留原有 provider 固定字段；新增字段默认可空，不破坏旧配置读取。
