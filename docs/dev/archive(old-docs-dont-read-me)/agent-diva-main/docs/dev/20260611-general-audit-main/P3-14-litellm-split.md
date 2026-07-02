# P3-14: LiteLLM provider 过大需拆分

## 问题描述

`agent-diva-providers/src/litellm.rs` 当前约 1601 行，集中了承担多类职责的代码：

- 第 21 行起定义 chat request/response DTO、stream chunk DTO、tool call DTO。
- 第 154 行起定义 `LiteLLMClient`，同时持有 HTTP client、provider registry、selected provider、模型解析和 reasoning 配置。
- 构造函数 `LiteLLMClient::new` 负责 provider lookup、api_base 推导、HTTP client 构建和默认 reasoning 清洗。
- `parse_response` 负责 OpenAI-compatible response 到 `LLMResponse` 的映射。
- `build_request` 负责 request body 组装、tool choice、stream 选项。
- `apply_headers`、`serialize_request_body`、`log_request_failure` 处理 HTTP 请求细节和日志。
- `impl LLMProvider` 中 `chat` 与 `chat_stream` 同时处理模型解析、override、cache control、HTTP、错误转换、JSON 解析和 stream 聚合。
- 文件底部还包含大量单元测试。

该文件已经超出单一 provider adapter 的合理边界，且 chat 与 stream 两条路径存在相似的 request 构建、header、错误处理和日志代码。

## 影响评估

- 可维护性影响：模型解析、HTTP 错误、stream 聚合、cache_control 任一改动都需要理解整个大文件。
- 回归风险：chat 与 chat_stream 的重复逻辑容易出现一边修复、一边遗漏。
- 测试定位困难：DTO 反序列化、request builder、stream parser、provider model resolution 的测试混在同一文件。
- 扩展成本：后续新增 OpenAI-compatible provider 特性时，文件会继续膨胀。

## 解决方案

按职责拆成内部模块，保持外部 `LiteLLMClient` API 不变。

建议结构：

```text
agent-diva-providers/src/litellm/
  mod.rs
  client.rs
  types.rs
  request.rs
  response.rs
  stream.rs
  errors.rs
  cache_control.rs
  model.rs
```

拆分边界：

- `types.rs`：`ChatCompletionRequest`、`ChatCompletionResponse`、`StreamChunk` 等 serde DTO。
- `request.rs`：`RequestBuildOptions`、`build_request`、message normalization、cache control 调用边界。
- `response.rs`：非流式 `parse_response`、tool call 参数解析。
- `stream.rs`：SSE/chunk buffer、partial tool call 聚合、`finalize_partial_response`。
- `errors.rs`：HTTP non-success response 解析、日志上下文、结构化 provider error 映射。
- `model.rs`：`resolve_model`、provider registry 相关 override。
- `client.rs`：`LiteLLMClient` struct、constructor、`impl LLMProvider` 编排。

迁移方式：

1. 先把 DTO 和纯函数移动到子模块，不改变行为。
2. 保留 `pub use client::LiteLLMClient;`，避免外部导入变更。
3. 将现有测试按模块迁移，保证每步可编译。
4. 再消除 chat/chat_stream 中重复代码，例如抽取 `prepare_body` 和 `send_chat_request`。

示例：

```rust
mod client;
mod request;
mod response;
mod stream;
mod types;

pub use client::LiteLLMClient;
```

## 验证方法

执行：

```powershell
cargo test -p agent-diva-providers litellm
cargo test -p agent-diva-providers
just fmt-check
just check
```

必须保持现有行为测试通过，尤其是：

- native provider 不自动添加 LiteLLM provider/model 前缀。
- request builder 保留 image_url multimodal parts。
- stream partial tool call 能正确合并。
- null `tool_calls` 和 null `usage` 仍反序列化为默认值。
- cache control 只对支持模型生效。

验收标准：

- 原 `litellm.rs` 缩小为模块入口或薄 client。
- chat 与 stream 共享 request/error 处理工具。
- 外部 crate 不需要修改导入路径。

## 优先级

P3
