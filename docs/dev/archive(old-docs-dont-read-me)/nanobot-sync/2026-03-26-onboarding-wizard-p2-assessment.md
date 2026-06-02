# P2 Onboarding Wizard 与渠道精细交互评估

本文评估两个候选方向的价值、优先级和最小实现工程量：

1. onboarding 继续向 `.workspace/nanobot/nanobot/cli/onboard.py` 的 wizard 体验靠拢。
2. 渠道侧精细交互，如 Slack done reaction、Feishu reply context、Telegram reply context、富文本细节。

结论先行：

- P2 更适合先做 onboarding wizard 增强，而不是先做渠道精细交互。
- `agent-diva` 已有可复用基础，不需要重写一套 nanobot 式系统，应该在现有 `agent-diva-cli/src/main.rs` 的 `run_onboard` 上增量演进。
- 最小可交付版本应聚焦“provider 导向的闭环配置 + 更强模型补全 + summary/确认保存”，不把问题扩展成通用表单引擎。

## 现状判断

### agent-diva 已有基础

当前 CLI onboarding 已具备以下能力：

- 选择 provider
- 输入 API Key / API Base
- 基于 provider 拉取模型列表或回退到静态模型
- 选择 workspace
- 保存配置并补齐 workspace 模板

对应位置：

- `agent-diva-cli/src/main.rs`
- `agent-diva-cli/src/cli_runtime.rs`
- `agent-diva-providers/src/discovery.rs`
- `agent-diva-providers/src/registry.rs`

这意味着 `agent-diva` 并不是“没有 onboard”，而是“还没有形成 wizard 闭环”。

### nanobot 的关键体验点

从 `.workspace/nanobot/nanobot/cli/onboard.py` 看，nanobot 的优势不在于字段更多，而在于流程更完整：

- 主菜单式分段配置，而不是单向线性问答
- 支持 section 内回退
- provider 单独配置，交互入口清晰
- model 字段有自动补全入口
- context window 等字段可根据 model 给推荐值
- summary 面板 + save/discard 闭环
- 配置修改与未保存状态有明确反馈

需要注意：

- nanobot 当前 `models.py` 里的模型补全和 context limit 查询本身还是占位实现
- 因此这次对标应该学“交互闭环”，不是照搬 nanobot 的内部技术实现

## 为什么 onboarding 更适合放在 P2 前半段

### 用户价值更直接

onboarding 是首次使用和配置变更时的第一触点。当前已有 provider/model 发现能力，但体验仍偏“串行表单”，用户感知不到系统已经有的能力。

相比之下，渠道细节优化虽然重要，但前提是：

- 用户已经完成配置
- 已经进入某个具体 channel
- 还要恰好命中对应交互场景

P2 阶段更应该优先解决“第一印象”和“配置成功率”。

### 复用现有能力更多

onboarding 增强可以直接复用：

- provider registry
- 运行时模型发现
- 默认模型推断
- 配置保存与模板同步

渠道精细交互则更分散，涉及：

- 每个 channel 单独接入
- inbound metadata 补充
- outbound 行为差异
- 每个渠道各自的测试夹具

因此 onboarding 的单位收益更高，工程风险更低。

## P2 最小实现建议

### 目标

把当前单段式 `run_onboard` 提升为“provider 导向的向导”，但不扩展成 nanobot 那种通用配置编辑器。

### 最小范围

建议只做以下 4 个能力：

1. 分步 wizard 骨架
2. 按 provider 引导配置
3. 更强的模型补全/推荐
4. summary + 确认保存闭环

### 具体形态

建议流程：

1. 入口页：检测已有配置，选择 refresh / overwrite / cancel
2. Provider 步骤：选择 provider，并展示 provider 默认 API Base、默认模型、是否支持模型发现
3. Credentials 步骤：输入 API Key / 可选 API Base，保留“保持现有值”
4. Model 步骤：
   - 先拉运行时模型列表
   - 若拉取成功，提供选择 + 手动输入双入口
   - 若拉取失败，使用 registry 静态模型作为补全候选
   - 对当前 provider 的默认模型做显式推荐
5. Workspace 步骤：展示默认 workspace，允许修改
6. Summary 步骤：展示本次变更摘要，确认保存或返回修改

### 不建议纳入最小范围的内容

- 通用 Pydantic/Rust schema 驱动表单引擎
- onboarding 中直接配置 channel
- context window / reasoning effort 等高级字段
- provider OAuth/login 闭环
- TUI/GUI 双端统一 wizard

这些都是真需求，但会把 P2 从“体验增强”放大成“配置系统重构”。

## 最小工程改动面

### `agent-diva-cli`

这是主战场。

建议新增一个独立模块，例如：

- `agent-diva-cli/src/onboard_wizard.rs`

职责：

- 定义 wizard step 状态
- 组装 provider/model/workspace 的交互流程
- 输出最终 `OnboardDraft`
- 统一做 summary 和 save

`main.rs` 中的 `run_onboard` 只保留入口编排。

### `agent-diva-cli/src/cli_runtime.rs`

复用现有接口，补少量辅助函数即可，例如：

- provider 显示信息组装
- 模型候选聚合与去重
- provider 默认说明文本

尽量不要把交互逻辑塞回 runtime。

### `agent-diva-providers`

大概率不需要新增核心机制。

只要确认现有 `fetch_provider_model_catalog` 的返回信息足够支撑：

- 运行时拉取成功
- 静态回退
- 来源标识

如果需要，可补极少量元信息暴露，但不建议在 P2 改 discovery 架构。

## 预估工程量

### 方案 A：严格最小版

范围：

- 单独抽模块
- provider 分步引导
- 模型候选增强
- summary/确认保存
- 基础测试

预估：

- 0.5 到 1 人日完成实现
- 0.5 人日补测试和收尾
- 总计约 1 到 1.5 人日

这是我认为最合理的 P2 最小落点。

### 方案 B：接近 nanobot 体验版

额外包含：

- 步骤内回退
- 未保存变更提示
- 更清晰的 section 菜单
- 更完整的 provider 描述信息

预估：

- 2 到 3 人日

这个版本体验更完整，但已经明显超出“最小实现”。

### 方案 C：通用配置向导版

额外包含：

- 通用字段编辑抽象
- channel/config/soul 等多 section 配置
- 更复杂状态管理

预估：

- 4 到 6 人日以上

不建议作为 P2。

## 渠道精细交互的评估

### 价值判断

这些能力都有价值，但不应先于 P0/P1，也不应挤占 P2 的 onboarding 主线：

- Slack done reaction
- Feishu reply context
- Telegram reply context
- 更细的富文本渲染

原因不是它们“不重要”，而是它们更适合在各 channel 进入稳定期后按渠道逐个补齐。

### 当前状态

当前 `agent-diva` 并非完全没有基础：

- Slack 已有 thread 元数据透传与 mrkdwn 转换
- Feishu 已有 markdown/table 卡片渲染，也有 seen reaction
- Telegram 已有 markdown 到 HTML 的发送渲染

但与 nanobot 相比，仍缺少更精细的“会话上下文拼接”和“完成态反馈”：

- Slack 缺 done reaction 闭环
- Feishu 未补 reply context 提取
- Telegram 未把 reply_to_message 语义注入 inbound content / metadata
- 富文本细节仍以单渠道各自实现为主，缺少一致性策略

### 最小工程量估计

如果只做单点增强：

- Slack done reaction：约 0.5 人日
- Feishu reply context：约 0.5 到 1 人日
- Telegram reply context：约 0.5 到 1 人日
- 单渠道富文本细节修补：约 0.5 到 1 人日 / 项

如果把这些打包成一个“渠道精细交互包”，实际成本通常会膨胀到 2 到 4 人日，因为测试、回归和渠道差异会明显放大。

## 推荐优先级

### P2-A

先做 onboarding wizard 最小版。

交付标准：

- provider 导向的分步流程
- 模型候选比现在更强
- summary/确认保存闭环
- 至少有 1 条 CLI smoke test

### P2-B

onboarding 稳定后，再从渠道精细交互里挑 1 个最值钱的点。

建议顺序：

1. Telegram reply context
2. Feishu reply context
3. Slack done reaction

原因：

- reply context 直接影响 agent 对“你回复的是哪条消息”的理解质量
- done reaction 更偏体验加分项，不如上下文正确性刚性

## 建议的落地方式

如果只允许做一个最小 P2，我建议这样定义范围：

“在不重构配置系统的前提下，把 CLI onboard 升级成 provider 导向 wizard，补齐模型候选、步骤确认和保存闭环；渠道精细交互仅做文档记录，不进入本次实现范围。”

这样做的好处是：

- 可以明显提升首次配置体验
- 能复用现有 provider/model 基础设施
- 不会打断 P0/P1 主线
- 工程量可控，回归面也较小

## 验收建议

最小验收：

- `agent-diva onboard` 可完整走完 provider -> credentials -> model -> workspace -> summary
- 已有配置时可以选择保留现值或覆盖
- 模型选择优先显示动态发现结果，失败时回退到静态列表
- 保存后 `agent-diva config doctor` 可通过基础检查

建议补充测试：

- onboarding 单元测试或流程测试，覆盖已有配置与空配置两条路径
- CLI smoke test，至少验证 wizard 完成后配置落盘成功
