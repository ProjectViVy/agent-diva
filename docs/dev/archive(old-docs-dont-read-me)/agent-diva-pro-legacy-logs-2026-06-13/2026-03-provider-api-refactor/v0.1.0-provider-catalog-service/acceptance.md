# Acceptance

1. 调用 `GET /api/providers`，应返回 builtin/custom 统一 provider 视图，而不是旧的 registry 原始结构。
2. 调用 `GET /api/providers/:id/models?runtime=true`，应返回统一模型目录，包含 `models`、`custom_models`、来源和错误信息。
3. 对 builtin provider 调用 provider-model 增删接口后，配置文件中的 `custom_models` 应更新。
4. 创建 custom provider 后，应可被 provider 列表、provider 解析和 provider set 识别。
5. 使用自定义 OpenAI-compatible provider 时，发送到 provider 原生 endpoint 的 model id 应保持原样，不自动加 LiteLLM 前缀。
