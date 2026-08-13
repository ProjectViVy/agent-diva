# Summary

- 打通了聊天主链路里的图片附件输入：`InboundMessage.media` 中的图片附件现在会在 `agent-diva-agent` 当前轮转换为结构化多模态消息，而不再只被文本占位化。
- 文本附件仍按原有策略内联或给出占位提示；图片附件则转为 data URI 形式的 `image_url` content part，并和当前用户文本合并到同一条 user message 中。
- 新增视觉能力门控：当当前模型不支持视觉输入时，agent 会在 provider 调用前明确报错并提示切换视觉模型。
- 同时修正了一个链路问题：turn 处理此前错误地始终读取 provider 默认模型，而不是 agent 当前配置模型。
