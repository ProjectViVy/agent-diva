# 架构设计探索

原始 DSML 在 Provider 层解码为既有 `LLMResponse`，Agent Loop 继续只处理结构化工具调用；默认 OpenAI JSON 路径不变。
