---
project_name: agent-diva-providers
date: 2026-03-30
module: agent-diva-providers
status: complete
parent_workspace: agent-diva
---

# agent-diva-providers — 项目上下文

## 模块职责

- **抽象层**：`base` 定义 `LLMProvider`（`chat` / `chat_stream`）、`Message`、`LLMResponse`、`LLMStreamEvent`、`ToolCallRequest`、`ProviderError` / `ProviderResult`、`ProviderEventStream`（`futures::Stream` + `Send`）。
- **LiteLLM 实现**：`litellm::LiteLLMClient` — `reqwest` 调用兼容 OpenAI 风格的 Chat Completions（含流式 chunk 解析、工具调用、reasoning 字段等），结合 `ProviderRegistry` 做模型前缀与供应商元数据。
- **注册表**：`registry` 内嵌 `providers.yaml`（`serde_yaml`），`ProviderSpec` / `ApiType`，按模型关键字或配置名解析供应商。
- **发现与目录**：`discovery` 从运行时配置拉取模型列表（`ProviderModelCatalog`、`ModelCatalogSource`、`ProviderAccess::from_config`）；`catalog` 提供目录服务与视图类型（UI/服务层聚合）。
- **热切换**：根模块 `DynamicProvider`（`RwLock<Arc<dyn LLMProvider>>`）实现 `LLMProvider`，转发到当前实现。
- **转写**：`transcription::TranscriptionService` — Groq Whisper API（multipart 上传，`TranscriptionError`）。

## 依赖与边界

| 依赖 | 用途 |
|------|------|
| `agent-diva-core` | `ErrorContext`、配置类型（如 `ProviderConfig`）等横切能力 |
| **workspace** | `tokio`、`async-trait`、`futures`、`serde`/`serde_json`、`serde_yaml`、`thiserror`、`reqwest`、`tracing`、`regex` |

**边界**：本 crate 负责「与 LLM 网关/供应商对话」的数据模型与客户端；不包含 Agent 循环、工具执行、会话存储。`anyhow` 非直接依赖（错误以 `ProviderError` / `thiserror` 为主）。

## 关键类型/入口

- `LLMProvider`：`async_trait`，所有具体客户端需 `Send + Sync`。
- `Message::{user,system,assistant,tool}`：构造标准角色消息；支持 `tool_calls`、`reasoning_content`、`thinking_blocks` 等扩展字段序列化。
- `LiteLLMClient`：默认/可配置 base URL 与密钥，对接 LiteLLM 或 OpenAI 兼容端点。
- `ProviderRegistry::new`：`include_str!("providers.yaml")` 解析失败会在初始化时 panic（构建期保证 YAML 合法）。
- `pub use`：`base`、`catalog`、`discovery`、`litellm`、`registry` 及 `DynamicProvider`（在 `lib.rs` 实现）。

## 实现约定

- 异步 HTTP 使用 **reqwest** + **Tokio**；trait 对象流使用 `Pin<Box<dyn Stream<...> + Send>>`。
- **tracing** 用于 debug/warn/error（请求与流解析路径）。
- **serde**：`ToolCallRequest` 自定义序列化/反序列化以兼容 `function.arguments` 字符串或对象；响应侧注意 null 与缺省字段（如 `deserialize_with` 处理）。
- **regex**：用于响应文本清理或模型/供应商相关规范化（见 `litellm`）。
- 新增供应商：优先实现 `LLMProvider` + 必要时扩展 `ProviderSpec` / YAML；复杂目录逻辑放在 `discovery`/`catalog`，避免把注册表与 HTTP 细节缠在一起。

## 测试与检查

- `registry` 模块含 `find_by_model` / 名称查找等单元测试。
- **dev-deps**：`tokio-test`、`mockito`（HTTP mock）。
- 修改后建议：`cargo test -p agent-diva-providers`、`cargo clippy -p agent-diva-providers`。

## 切勿遗漏

- 修改 `src/providers.yaml` 后须保证能通过 `serde_yaml` 解析，否则 `ProviderRegistry::new` 在测试/运行时直接 panic。
- `DynamicProvider::update` 在写锁失败时静默不更新（当前实现用 `if let Ok`）；调用方若依赖强一致需知悉。
- 流式路径与聚合路径（`chat` vs `chat_stream`）行为需与 `agent-diva-agent` 侧消费方式一致（事件类型 `LLMStreamEvent::Completed` 等）。
- `discovery` 依赖网络与配置时，失败应体现在 `ProviderModelCatalog::source` / `error` 字段，而非在库内隐式 fallback 到错误数据。
