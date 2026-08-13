# DeepSeek V4、DSML 与 Agent Diva Agent 兼容性调研

> 调研日期：2026-07-12。本文用于选择部署适配策略，并定位“DSML 工具调用被显示给用户”的故障边界；不是对某个网关兼容性的承诺。

## 结论摘要

DeepSeek V4 有两条不能混淆的集成路径：

1. **官方托管 API**：程序收发 OpenAI-compatible 的 `content`、`reasoning_content` 和结构化 `tool_calls`。Agent Diva 现有 `OpenAiCompatibleClient` 适用于这条路径。
2. **本地 V4 权重或未完成协议转换的网关**：模型原始文本使用 DSML 表达工具调用。该文本必须先经 V4 专用编码器/解析器转换为结构化响应，不能直接交给通用 Agent Loop。

近期的协议泄漏修复是安全边界：它会阻止 DSML 进入 GUI，也不会执行其中的调用。它不等价于 DSML 执行兼容。若需要直接接入原始 V4 文本流，应新增显式的 `deepseek_v4_dsml` 适配层，而不是放宽当前的泄漏拦截规则。

## 一、模型与部署事实

DeepSeek V4-Pro（1.6T 总参数、49B 激活）和 V4-Flash（284B 总参数、13B 激活）都支持 1M token 上下文；模型卡同时列出 non-think、high、max 三档推理模式。模型卡建议本地部署使用 `temperature=1.0`、`top_p=1.0`，Think Max 至少配置 384K context window。[DeepSeek V4-Pro 模型卡](https://huggingface.co/deepseek-ai/DeepSeek-V4-Pro)

最重要的部署差异是：V4 发布物**没有**通用 Jinja chat template，而是提供 `encoding/` 中的 Python 编码/解码参考实现与测试。因此不能把 Qwen、Llama 或 V3 的模板套到 V4 权重上。模型卡给出了 vLLM、SGLang 的 OpenAI-compatible 服务示例，但服务端仍须完成 V4 的消息编码与输出解析。[模型卡的 Chat Template 与本地部署说明](https://huggingface.co/deepseek-ai/DeepSeek-V4-Pro)

### 建议的部署选择

| 运行方式 | Agent Diva 接口预期 | 必要条件 | DSML 责任方 |
| --- | --- | --- | --- |
| `https://api.deepseek.com` | 标准 OpenAI JSON | 原始模型 ID、保留 reasoning/tool 历史 | DeepSeek API |
| vLLM/SGLang 已启用 V4 parser | 标准 OpenAI JSON | V4 tool + reasoning parser 均启用 | 推理服务 |
| 原始 Transformers/自定义网关 | 原始生成文本 | 集成官方 `encoding_dsv4.py` 等价实现 | Agent Diva 或该网关 |
| 不确定的兼容网关 | 先做探测 | 捕获完整非流式与流式响应 | 由探测结果决定 |

## 二、DSML 协议的实际含义

官方 `encoding/README.md` 将 `｜DSML｜` 定义为特殊标记。工具存在时，会把工具 schema 注入提示词；模型以 `<｜DSML｜tool_calls>`、`invoke` 与 `parameter` 块输出调用。参数的 `string="true"` 表示原始字符串，`string="false"` 表示数值、布尔、数组或对象的 JSON 值。工具执行结果被包装为用户消息中的 `<tool_result>`，多个结果按前一轮调用顺序回填。[V4 encoding README：特殊 token、工具格式与结果回填](https://huggingface.co/deepseek-ai/DeepSeek-V4-Pro/blob/main/encoding/README.md)

因此如下文本并不是用户可见回答，而是模型到协议适配器的中间表示：

```text
<｜DSML｜tool_calls>
<｜DSML｜invoke name="write_file">
<｜DSML｜parameter name="path" string="true">tests/test_cli.py</｜DSML｜parameter>
...</｜DSML｜invoke>
</｜DSML｜tool_calls>
```

V4 的 thinking 模式还要求 `<think>...</think>` 出现在工具调用或最终回答之前。启用工具时，参考编码器会保留前面轮次的 reasoning；无工具对话才允许依据 `drop_thinking` 去除较早的 reasoning。`reasoning_effort="max"` 通过 prompt 最前部的特殊指令提升推理深度。[V4 encoding README：thinking、`drop_thinking` 与 effort](https://huggingface.co/deepseek-ai/DeepSeek-V4-Pro/blob/main/encoding/README.md)

## 三、官方编码器是集成规范

`encoding_dsv4.py` 的 `encode_messages()` 将 OpenAI 风格消息、工具结果、多轮上下文、thinking 状态和 reasoning effort 编码为 V4 prompt；其 `parse_message_from_completion_text()` 从原始 completion 提取 `reasoning_content`、`content` 与 OpenAI 格式 `tool_calls`。[参考实现：编码入口与参数](https://huggingface.co/deepseek-ai/DeepSeek-V4-Pro/blob/main/encoding/encoding_dsv4.py) [参考实现：completion 解析入口](https://huggingface.co/deepseek-ai/DeepSeek-V4-Pro/blob/main/encoding/encoding_dsv4.py)

解析器的安全特性和限制同样重要：它检查参数格式、拒绝重复参数名；对于不正确的 completion 明确抛出 `ValueError`，而非自动修复。这意味着生产系统应在其外再增加：流式状态机、最大缓冲大小、工具白名单、schema 校验、错误分类以及 fail-closed 的用户输出策略。

## 四、官方 API 与原始 DSML 的边界

官方 Chat Completion API 暴露的是结构化 `tool_calls`；`tool_choice: "none"` 要求模型生成消息而不调用工具，`auto`、`required` 和指定函数则有各自语义。官方仍要求调用方验证工具参数，因为模型可能给出无效 JSON 或 schema 外参数。[DeepSeek Chat Completion：`tools`、`tool_choice` 与响应结构](https://api-docs.deepseek.com/api/create-chat-completion/)

Thinking + tool calling 的关键约束：发生工具调用的轮次，`reasoning_content` 必须完整回传给后续 API 请求；否则 API 返回 400。仅有普通 thinking 对话而未发生工具调用时，旧 reasoning 不必保留。[DeepSeek Thinking Mode：多轮工具调用规则](https://api-docs.deepseek.com/guides/thinking_mode/)

这解释了两个常见误判：

- 官方 API 中看见 JSON `tool_calls` 是正常行为，不能当 DSML 泄漏处理。
- 在本地输出里看见 DSML 也未必是模型异常；它可能只是服务端缺少 V4 parser，或者错误地把原始 completion 放进 OpenAI 的 `content`。

## 五、vLLM 参考架构

vLLM 当前文档将 V4 parser 描述为“在一个状态机中解析 `<think>`/`</think>` reasoning 和 DSML tool calls”，并展示 V4 使用 `<｜DSML｜tool_calls>` 的格式。[vLLM DeepSeek V4 parser](https://docs.vllm.ai/en/latest/api/vllm/parser/deepseek_v4/)

对 Agent 集成而言，重点不是仅配置 tool parser：reasoning parser、tool parser 和流式状态都必须协作。流式输出在 DSML 起始标签未判定前不应推送给 UI；一旦解析为调用，应该发出结构化工具 delta 或等待完整调用，不应发送为 `content`。V4 与旧版 DSML 的外层标签不同，复用不匹配的 V3.x parser 有将调用降格为文本的风险。

## 六、Agent Diva 当前状态与故障定位

现有 `agent-diva-providers/src/openai_compatible.rs` 将 OpenAI JSON 的 `message.tool_calls` 转成内部 `ToolCallRequest`，并保留 `reasoning_content`；它没有 DSML 文本解析器。因此当上游已正确完成转换时，正常工作；当上游把 DSML 写入 `content` 时，原先会被 Agent Loop 当作回答。

提交 `1250e79` 增加了两层保护：

- 无工具请求显式发送 `tool_choice: "none"`；
- 流式 `AssistantDelta` 和最终 `FinalResponse` 检测 DSML/XML 风格内部协议，并改用确定性迭代上限摘要，拒绝执行该文本中的调用。

这解决“协议泄漏到 GUI”的安全问题，但不会执行原始 DSML。故障链应按以下顺序观测：

```text
OpenAI messages + tools
  -> V4 prompt encoder
  -> model raw completion (think / DSML)
  -> reasoning + DSML parser
  -> OpenAI content / reasoning_content / tool_calls
  -> Agent Diva tool executor and GUI
```

若 GUI 看见 DSML，优先捕获“模型原始输出”和“HTTP 响应 JSON”，确认问题发生在推理服务 parser 之前或之后；不要先修改工具执行器。

### 对既有防泄漏更新的处理

不应回滚提交 `1250e79` 的安全保护。它阻止未解析的内部协议抵达 GUI，且在 summary-only 阶段防止模型越过迭代上限继续执行工具；这两项不变量应保留。

后续兼容改动应把该保护从“全局拦截 DSML”演进为“**未被指定协议适配器成功解析的 DSML 才拦截**”：

```text
DeepSeek V4 原始流
  -> DSML 流式解析器
  -> ToolCallRequest / ReasoningDelta / TextDelta
  -> Agent Loop
  -> 未解析协议的最终泄漏拦截
```

- `openai_json`（默认）继续采用当前 fail-closed 策略：任何出现在 `content` 的 DSML 都是上游转换失败，必须拦截。
- `deepseek_v4_dsml` 必须显式配置；解析成功的 DSML 变为既有结构化工具调用，不再触发泄漏拦截。
- 解析失败、畸形调用、未知工具以及 summary-only 阶段的 DSML 一律保持拒绝执行和确定性状态摘要。
- 当前实现把所有无工具请求都序列化为 `tool_choice: "none"`。为兼容可能拒绝该字段的非标准本地网关，后续应将 Provider 请求的工具模式显式化，并仅在 summary-only 请求中发送 `none`；正常的无工具聊天保留省略该字段的现有兼容行为。

## 七、建议的兼容实现与验收矩阵

若产品需要接入原始 DSML，上层配置应显式区分 `openai_json`（默认）与 `deepseek_v4_dsml`，禁止自动猜测协议。`deepseek_v4_dsml` 适配器应：

1. 用官方参考实现等价的语法转换完成消息编码和 completion 解码；
2. 将成功解析的调用转换为现有 `ToolCallRequest`，保留完整 `reasoning_content`；
3. 对畸形、未知工具、重复参数、超限缓冲、schema 不合法的调用返回协议错误，不执行；
4. 在 summary-only 时禁止工具，并把任何结构化调用或 DSML 都转为确定性状态摘要；
5. 仅在解析确认是用户文本后，才把流式 token 发给 GUI。

最低回归矩阵：

| 场景 | 预期 |
| --- | --- |
| 官方 API 的 JSON `tool_calls` | 正常执行并回填 tool result |
| 完整 DSML 单调用 | 转为一次内部工具调用 |
| 并行 DSML 调用 | 保持顺序与 call ID 映射 |
| DSML 分跨多个 stream chunk | 不向 GUI 泄漏半截标签 |
| `<think>` + DSML | reasoning 与 tool call 分离并保留后续所需 reasoning |
| `string=true/false`、数组、对象、换行字符串 | 按官方语义恢复参数 |
| 重复/未知参数、嵌套 invoke、未闭合标签 | 拒绝执行，输出安全错误/状态摘要 |
| `tool_choice: none` 或 summary-only | 不执行工具，绝不显示 DSML |

## 参考资料

- [DeepSeek V4-Pro 模型卡、部署与 encoding 目录](https://huggingface.co/deepseek-ai/DeepSeek-V4-Pro)
- [DeepSeek V4 encoding README](https://huggingface.co/deepseek-ai/DeepSeek-V4-Pro/blob/main/encoding/README.md)
- [DeepSeek V4 `encoding_dsv4.py` 参考实现](https://huggingface.co/deepseek-ai/DeepSeek-V4-Pro/blob/main/encoding/encoding_dsv4.py)
- [DeepSeek Tool Calls 文档](https://api-docs.deepseek.com/guides/tool_calls)
- [DeepSeek Chat Completion 参考](https://api-docs.deepseek.com/api/create-chat-completion/)
- [DeepSeek Thinking Mode 文档](https://api-docs.deepseek.com/guides/thinking_mode/)
- [vLLM DeepSeek V4 parser](https://docs.vllm.ai/en/latest/api/vllm/parser/deepseek_v4/)
