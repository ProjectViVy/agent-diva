# Summary

- 目标：修复 `agent-diva-gui` 设置 -> 供应商逻辑中对“网关 / 本地 / 云端”的分类分支依赖，统一改为按标准 provider 元数据、`api_type`、`api_base` 与原始 model id 处理。
- 实现：
  - 删除共享 provider 视图、CLI 状态、Tauri bridge、GUI 类型中的 `is_gateway` / `is_local` 分类字段。
  - 删除后端 provider registry 中基于“网关 / 本地”分类的查找与判定逻辑，不再按该分类过滤模型归属。
  - 调整 `LiteLLMClient` 的模型解析逻辑：
    - 命中 provider 原生 `api_base` 时，发送原始 model id；
    - 仅在显式命名 provider 且实际请求 base 不是该 provider 原生 base 时，才回退到 LiteLLM 前缀模式。
  - 调整 `agent-diva-manager` 的配置更新归一化逻辑：
    - 当本次更新请求里 `provider` 和 `model` 都是显式给定时，保留用户传入的 model；
    - 仅在 `model` 不是本次显式指定时，才回退到 provider 默认模型。
  - GUI 设置页不再展示“本地 / 网关 / 云端”标签，统一展示 `api_type` / 标准 API 风格。
- 影响：
  - 供应商设置页的前后端行为不再因 provider 被标注为 gateway/local 而走特殊路径。
  - 原生 OpenAI-compatible / Anthropic 风格 provider 与此前“网关”类 provider 采用一致的基础处理方式。
  - 修复了 `silicon` 在显式选择 `ByteDance-Seed/Seed-OSS-36B-Instruct` 时被静默回退到 `deepseek-ai/DeepSeek-V3.2` 的问题。
  - 同类依赖“provider 关键词匹配”而非显式 model 保留的 provider 也一并受益，例如 `openrouter`、`aihubmix`、`302ai`、`ph8`、`burncloud`、`silicon`、`ppio`、`together`、`ocoolai`、`modelscope`、`hyperbolic`、`infini`、`qiniu`、`tokenflux`、`cephalon`、`lanyun`、`poe`、`aionly` 等。
