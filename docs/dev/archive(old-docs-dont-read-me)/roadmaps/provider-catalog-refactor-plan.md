# Provider Catalog 重构与自定义 Provider/Model 计划

## 1. 背景与问题

当前 provider 相关代码已经出现明显的结构性膨胀，主要体现在：

- **静态 registry 与运行时配置割裂**
  - `agent-diva-providers/src/providers.yaml` 提供内建 provider 元数据。
  - `agent-diva-core` 的 `ProvidersConfig` 仍然是固定字段结构。
  - 两者之间没有统一的合并视图，导致“内建 provider 元数据”和“用户配置 provider 实例”不是同一层概念。

- **provider 槽位是硬编码的**
  - `ProvidersConfig` 使用固定字段：`openai`、`deepseek`、`openrouter`、`custom` 等。
  - CLI、manager、Tauri 都有重复的 `match name { ... }` 映射逻辑。
  - 每新增一个 provider，都要同步改多处代码。

- **provider 与 model 逻辑没有统一入口**
  - provider 列表、provider 查询、模型目录、provider 解析、provider access、运行时模型发现分别散落在不同 crate。
  - GUI / CLI / Manager API 的 DTO 和视图层又各自再拼一遍。

- **当前结构不适合扩展自定义能力**
  - 自定义模型只能临时挂在现有 provider 下面。
  - 自定义 provider 若继续沿用现有固定槽位思路，只会把 `ProvidersConfig` 与各处 `match` 继续放大。

结论：如果只是继续给现有结构追加 `custom_models`、`custom_provider` 特例字段，短期能跑，长期仍会继续变臃肿。

## 2. 本次目标

本次计划目标分为两层。

### 2.1 功能目标

- 支持在内建 provider 下添加/删除自定义模型。
- 支持新增/编辑/删除自定义 provider。
- 首版自定义 provider 限定为 **OpenAI-compatible** 协议。
- GUI、CLI、Manager API 三个入口统一可读写、可展示。

### 2.2 架构目标

- 把 provider 元数据、用户配置、模型目录、provider 解析、provider access 收敛为统一层。
- 尽量消灭跨 crate 的 provider 名称硬编码。
- 为后续 provider 扩展、模型扩展、运行时模型发现和配置迁移建立统一抽象。

## 2.3 GUI 需求整理

基于当前讨论，GUI 侧的需求可以整理为下面几类。

### A. Provider 列表与基础管理

- 现有内建 provider 继续显示在 Provider 设置页。
- 用户可以新增自定义 provider，而不只是使用固定的 `custom` 槽位。
- 自定义 provider 需要支持：
  - 新建
  - 编辑
  - 删除
- GUI 中 provider 列表需要明确区分：
  - 内建 provider
  - 用户自定义 provider

### B. 模型管理

- 用户可以在任意 provider 下手动添加模型。
- 手动添加模型后，需要：
  - 立即显示在当前 provider 模型列表里
  - 可以加入快捷切换列表
  - 可以直接切换为当前模型
- 手动添加的模型需要有显式标识，例如：
  - `Custom`
  - `Manual`
- 用户需要能删除手动添加的模型，而不是只能取消勾选快捷项。

### C. 持久化行为

- 自定义模型不能只保存在 GUI 本地。
- 自定义模型需要至少同时写入：
  - 项目配置
  - GUI 本机快捷列表
- 自定义 provider 需要写入项目配置，不能只存在于内存或 GUI 本地存储中。

### D. 模型来源与目录展示

- Provider 模型列表不能只显示静态 registry 模型。
- 需要统一展示以下来源合并后的模型目录：
  - 内建静态模型
  - runtime 在线发现模型
  - 手动添加模型
- GUI 最好能展示模型来源标签，避免用户不知道该模型来自：
  - Live catalog
  - Static fallback
  - Custom / Manual

### E. 选择与回退行为

- 添加模型后，可以直接设为当前模型。
- 删除当前正在使用的自定义模型时，GUI 需要有明确的回退策略：
  - 优先回退到该 provider 默认模型
  - 若无默认模型，则回退到该 provider 的第一个可用模型
  - 若 provider 已无任何可用模型，则保留错误提示，不静默失败
- 删除当前 provider 时，也需要定义 GUI 的当前选择回退逻辑。

### F. 交互体验

- Provider 设置页不应继续把复杂逻辑散在组件里临时拼接。
- GUI 应尽量只消费统一后的 provider DTO / catalog DTO。
- GUI 不应再自己推断：
  - provider 是否可用
  - model 是否属于 custom
  - 当前 provider/model 如何解析

## 2.4 其他未尽事宜整理

除 GUI 明确需求外，目前还有一组未尽事宜，需要在正式实施前纳入范围控制。

### 1. CLI 语义同步

- 如果 GUI 支持 custom provider / custom model，CLI 不能继续只认识固定 provider 槽位。
- 至少需要保证：
  - `provider list`
  - `provider status`
  - `provider models`
  - `provider set`
  对自定义 provider 有一致行为。

### 2. Manager API / Tauri 接口统一

- 当前 GUI 有一部分依赖 manager API，一部分依赖 Tauri command。
- 如果 provider 逻辑继续双轨维护，代码量只会继续上涨。
- 需要明确：
  - Manager API 是否成为唯一 provider 数据来源
  - Tauri 是否只做桥接
  - 还是两者都复用同一 provider service

推荐：**Manager API 与 Tauri 都复用统一 provider service，但 GUI 只消费一套统一 DTO。**

### 3. 配置迁移

- 无论是引入 `custom_models`，还是引入 `custom_providers`，都需要考虑：
  - 旧配置 load 是否兼容
  - 新配置 save 后是否仍能被旧逻辑最小程度容忍
  - migration crate 是否需要补迁移逻辑

### 4. provider 解析优先级

- 当前 provider 解析混合了：
  - 显式 provider
  - model 前缀
  - registry keyword 推断
- 引入 custom provider 后，必须重新定义优先级，否则会出现误解析。

建议优先级：

1. 显式 provider id
2. 当前激活 provider
3. model 前缀命中 provider id
4. 内建 registry 的 keyword 推断

### 5. 运行时模型发现边界

- 首版自定义 provider 只支持 OpenAI-compatible。
- 这件事要体现在 GUI、CLI、错误信息、文档和验收里。
- 不能在 UI 上把 custom provider 做成“什么都支持”的样子，但实际只有 OpenAI path 能跑。

### 6. DTO 与状态来源收敛

- 现在 provider 相关结构在多个层里重复定义。
- 后续必须限制新增 DTO 的数量，避免继续出现：
  - CLI 一套
  - Manager 一套
  - Tauri 一套
  - GUI 一套

目标应当是：

- Rust 内部统一 runtime/provider view
- 对外尽量只暴露一套稳定 DTO

### 7. 文档与用户说明

- 增加自定义 provider 后，用户文档需要同步说明：
  - 内建 provider 与自定义 provider 的区别
  - 自定义 provider 首版只支持 OpenAI-compatible
  - 模型来源有哪些
  - 删除当前模型 / 当前 provider 时的回退逻辑

## 3. 推荐方案：先统一 Catalog，再交付功能

这是本次**推荐执行路径**，适合控制风险并逐步收敛代码量。

### 3.1 最终目标抽象

新增一层统一的 **Provider Catalog / Provider Config Service**，提供以下能力：

- 列出当前可用 provider（内建 + 用户自定义）
- 按 name/id 查找 provider
- 按 model + preferred provider 解析 provider
- 返回 provider 的有效 access（api key / base / headers）
- 返回 provider 的合并模型目录（静态 / runtime / custom）
- 对 provider / model 做 CRUD

所有 CLI / Manager / Tauri / GUI 不再自己拼 provider 逻辑，而统一依赖这一层。

### 3.2 配置层最小演进

在保持旧配置兼容的前提下，先做最小演进：

```json
{
  "providers": {
    "openai": {
      "api_key": "",
      "api_base": null,
      "extra_headers": null,
      "custom_models": []
    },
    "deepseek": {
      "api_key": "",
      "api_base": null,
      "extra_headers": null,
      "custom_models": []
    },
    "custom_providers": {
      "my-proxy": {
        "display_name": "My Proxy",
        "api_type": "openai",
        "api_key": "",
        "api_base": "https://example.com/v1",
        "default_model": "foo-chat",
        "models": ["foo-chat", "foo-reasoner"],
        "extra_headers": {
          "x-app-id": "demo"
        }
      }
    }
  }
}
```

### 3.3 关键设计点

- **内建 provider registry 仍保留在 `providers.yaml`**
  - 作为只读 catalog 元数据来源。

- **用户自定义 provider 存在 config**
  - 作为运行时补充 catalog。

- **custom models 仍是 provider 级**
  - 内建 provider 放在 `ProviderConfig.custom_models`
  - 自定义 provider 放在 `CustomProviderConfig.models`

- **统一 ProviderView**
  - 无论内建还是自定义，统一映射成同一套运行时 view / DTO。
  - GUI / CLI / Manager API 不再关心底层来自 YAML 还是 config。

- **首版只支持 OpenAI-compatible 自定义 provider**
  - 原因：当前 `LiteLLMClient`、模型发现、provider 解析都围绕这一路径最成熟。
  - 不在首版同时打通 Anthropic / Google 原生协议。

## 4. 具体实施分期

### Phase 1：抽象收口，不改功能

- 在 provider 层新增统一 service，例如：
  - `ProviderCatalogService`
  - `ProviderRuntimeView`
  - `ProviderModelEntry`
- 把当前这些逻辑统一收口：
  - provider 列表
  - provider 查询
  - provider access
  - 当前 provider 解析
  - provider 模型目录
- CLI、manager、Tauri 都改为调用统一 service。
- 删除或下沉重复的 `provider_config_by_name` / `match` 分发逻辑。

**阶段目标**：不新增用户可见功能，只把 provider 逻辑收敛成一层。

### Phase 2：custom models

- 在内建 provider 下支持 `custom_models`。
- GUI provider 设置页支持手动添加 / 删除模型。
- 添加时：
  - 写入项目配置
  - 写入本机快捷模型
  - 设为当前模型
- 删除时：
  - 从项目配置与本机快捷列表都移除
  - 若是当前模型，则回退到该 provider 默认模型；没有默认模型则回退到合并目录第一项。
- CLI `provider models` 与 Manager API 返回合并目录。

### Phase 3：custom providers

- GUI 新增 provider CRUD：
  - 新增自定义 provider
  - 编辑连接参数
  - 删除 provider
- CLI 支持：
  - `provider list`
  - `provider status`
  - `provider models`
  - `provider set --provider <custom-id>`
- Manager API / Tauri 暴露统一的 provider CRUD 接口。
- 自定义 provider 与内建 provider 一样，参与：
  - 当前 provider 选择
  - 当前 model 选择
  - runtime model discovery
  - GUI 快捷切换

### Phase 4：删减旧逻辑

- 清理旧的分散 provider helper
- 清理只服务固定 provider 槽位的冗余函数
- 统一 DTO，避免 GUI / CLI / Manager 各自定义一份 provider 视图结构

## 5. 代码量缩减点

这部分是本次计划里最重要的“减法”，目标是**不是把代码搬家，而是真正减少重复代码**。

### 5.1 当前可直接削减的重复点

- CLI `provider_config_by_name` / `provider_config_by_name_mut`
- Manager `provider_config_by_name`
- Tauri `provider_config_by_name`
- Manager 中按 provider 名分发配置槽位的 `match spec.name.as_str()`
- GUI / Tauri / CLI 各自定义 provider list / catalog DTO 的重复拼装

### 5.2 建议统一后的最小公共接口

建议所有上层只依赖下面这组接口：

- `list_provider_views()`
- `get_provider_view(id)`
- `resolve_provider_id(model, preferred_provider)`
- `list_provider_models(id, runtime: bool)`
- `get_provider_access(id)`
- `save_provider_instance(...)`
- `delete_provider_instance(id)`
- `add_provider_model(id, model)`
- `remove_provider_model(id, model)`

只要这组接口稳定，上层 UI/CLI/API 基本不需要知道 provider 是内建还是自定义。

### 5.3 DTO 统一

统一两类 DTO：

- `ProviderView`
  - `id`
  - `display_name`
  - `source` (`builtin` / `custom`)
  - `api_type`
  - `default_model`
  - `api_base`
  - `configured`
  - `ready`
  - `runtime_supported`

- `ProviderModelCatalogView`
  - `provider`
  - `source`
  - `runtime_supported`
  - `api_base`
  - `models`
  - `custom_models`
  - `warnings`
  - `error`

这样可以减少 GUI/Tauri/Manager/CLI 四层的重复 struct 和字段映射代码。

## 6. 更轻量、更灵活的替代方案：颠覆当前配置逻辑

如果目标不是“平滑兼容旧结构”，而是**最大化缩减代码量并提升灵活性**，那更激进、也更值得考虑的方案其实是：

## 方案 B：放弃固定 provider 槽位，改为“provider instances” 模型

### 6.1 核心思路

把现在这种：

- `providers.openai`
- `providers.deepseek`
- `providers.openrouter`
- `providers.custom`

改成统一的实例配置：

```json
{
  "providers": {
    "default": "deepseek-main",
    "instances": {
      "deepseek-main": {
        "kind": "builtin",
        "builtin_ref": "deepseek",
        "api_key": "",
        "api_base": "https://api.deepseek.com/v1",
        "extra_headers": {},
        "models": ["deepseek-chat", "deepseek-reasoner"]
      },
      "corp-openai-proxy": {
        "kind": "custom",
        "api_type": "openai",
        "display_name": "Corp OpenAI Proxy",
        "api_key": "",
        "api_base": "https://llm.company.internal/v1",
        "default_model": "gpt-4o-mini",
        "models": ["gpt-4o-mini", "gpt-4o"]
      }
    }
  }
}
```

### 6.2 这个方案的优点

- 不再需要固定字段 `openai/deepseek/...`
- 不再需要 provider 名称到 config 槽位的 `match`
- builtin provider 与 custom provider 共享同一套数据结构
- 一套 CRUD 覆盖全部 provider
- `agents.defaults.provider` 可以直接指向实例 id，而不是“provider 类型名”
- 后续想支持多套 OpenAI / 多套 DeepSeek / 多个公司代理都很自然

### 6.3 这个方案的代价

- 配置迁移要更重
- 旧代码里大量默认假设需要改成“provider instance”
- 文档、CLI 语义、GUI 语义都要同步调整

### 6.4 我的判断

如果你愿意接受一次更大的配置迁移，这个方案实际上比“在现有固定槽位上继续叠加功能”**更轻、更干净、更长寿**。

换句话说：

- **保守路线**：先做 Provider Catalog Service，兼容旧结构，再渐进演进
- **激进路线**：直接切到 `provider instances` 配置模型

从“尽量缩减这块代码量”的角度，**激进路线更优**。

## 7. 推荐决策

我建议按下面的策略执行：

### 推荐主线

- **目标架构采用 `provider instances` 作为长期方向**
- **短期实施先做兼容层**
  - 先引入统一 provider catalog/service
  - 内部把旧固定槽位映射成 runtime provider instances view
  - GUI / CLI / Manager 先全部切换到 runtime view
  - 再决定 config 持久化是继续写旧结构，还是逐步迁到新结构

### 为什么这样最稳

- 可以先把最臃肿的 provider 逻辑收口，马上减掉重复代码
- 又不会在第一步就强制打爆旧配置兼容
- 等 provider 运行时统一后，再做配置迁移时风险小很多

## 8. 验证与验收建议

### 功能验收

- GUI 能新增、编辑、删除 custom provider
- GUI 能为内建 provider 添加、删除 custom model
- CLI 能列出并切换到 custom provider
- Manager API 返回统一 provider 视图和模型目录
- runtime model discovery 能对 custom OpenAI-compatible provider 生效

### 架构验收

- provider 相关 `match provider name` 逻辑显著减少
- provider DTO 数量减少
- CLI / Tauri / Manager 不再各自维护一套 provider 解析逻辑

### 验证命令

- `just fmt-check`
- `just check`
- `just test`
- `npm run build`（`agent-diva-gui`）

## 9. 最终建议

如果只是想“先把功能做出来”，推荐走 **Catalog 收口 -> custom models -> custom providers** 这条主线。

如果你的优先级是“这一块尽可能轻量化，少写重复代码，未来不再反复返工”，那么真正值得做的是：

**尽快把 provider 固定槽位配置逻辑，演进为 provider instances 逻辑。**

这一步才是最接近“颠覆现有配置逻辑”的方案，也是从根上减少 provider 代码量的方案。
