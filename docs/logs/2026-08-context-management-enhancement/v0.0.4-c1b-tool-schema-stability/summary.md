# C1b Tool Schema Stability Summary

## 完成内容

- 为工具定义增加 `Core` / `Deferred` 显式分区，保持 `register()` 默认 CORE 的兼容语义。
- 将内置工具保持为 CORE；MCP、custom/extension 工具显式装配到 DEFERRED 后缀。
- `get_definitions()` 固定按分区与工具名排序，并递归规范化 JSON object 键序。
- 将 C1-0 的“集合稳定、顺序未冻结”刻画升级为 T4 完整序列化字节契约。

## 影响范围

- `agent-diva-tooling` 的 registry 元数据、schema 序列化及公开分区类型。
- `agent-diva-agent` 的 ToolAssembly、custom tool builder 与 MCP 热更新注册路径。

## 明确未改

- 未引入 per-session `ToolSchemaCache`；当前 schema 均在工具实例内静态生成或持有。
- 未修改 SessionStable section cache、cache hash/usage 观测或 provider
  `apply_cache_control`；这些分别属于 C1c/C1d。
