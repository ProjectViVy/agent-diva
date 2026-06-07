# 2026-03-26 `docs/dev` 今日研究总结

本文汇总 `docs/dev` 于 2026-03-26 新增的 5 份研究文档，目标不是重复原文，而是抽取今天已经形成的共识、冲突点、优先级和建议执行顺序，供后续产品与工程排期直接使用。

## 研究范围

纳入本次总结的文档：

- `2026-03-26-nanobot-gap-analysis.md`
- `2026-03-26-provider-login-delivery-plan.md`
- `2026-03-26-plugin-architecture-reassessment.md`
- `2026-03-26-clawhub-registry-integration-plan.md`
- `2026-03-26-onboarding-wizard-p2-assessment.md`

## 一句话结论

今天的研究已经把方向收敛得比较清楚：`agent-diva` 当前最缺的不是基础 Agent 能力，而是几条面向用户的产品闭环与扩展闭环，尤其是 `provider login`、统一登录入口、插件机制和公共技能分发；`onboarding wizard` 则更适合作为这些底层闭环之上的 P2 体验增强项。

## 今日形成的核心共识

### 1. 当前主要缺口是“闭环”，不是“能力名词”

`agent-diva` 已经具备大量基础设施，包括 MCP、Cron、Heartbeat、Subagent、技能系统、多通道和 Web 工具。与 `.workspace/nanobot` 的差距，更多体现在：

- 文档和 CLI 已暴露，但能力尚未真正可用
- 扩展点仍偏静态编译，缺少统一外部扩展机制
- 多个子系统各自可用，但尚未形成统一产品路径

因此，后续开发不应再把重点放在“再补一层 Agent Loop”，而应放在“让已有能力形成真实可用路径”。

### 2. `provider login` 是最明确的 P0

5 份文档中最一致、最明确的结论就是：`agent-diva provider login <provider>` 已经公开存在，但仍是 placeholder，这属于当前最需要止血的产品断层。

其中：

- `openai-codex` 是最适合优先补齐的首个目标
- 认证模型应在 provider metadata 层显式表达，而不是继续把 OAuth 塞进 API-key 路径
- 凭据存储应与 `config.json` 解耦
- `qwen` 当前不应混入本轮 OAuth 登录实现

这项工作既能修正文档与实现不一致的问题，也会直接为后续 onboarding、provider status、provider models 提供真实基础。

### 3. 登录能力不应继续散落，应该抽象成统一入口

研究同时指出，登录不只是 provider 问题，channel 侧也存在相同模式：

- provider 侧需要统一 `provider login`
- channel 侧需要统一 `channels login`

因此，今天的共识不是“补一个 `openai-codex` 特例就结束”，而是应逐步建立统一的认证/登录入口，把“是否支持登录、用什么认证模式、凭据如何持久化”上提到共享抽象层。

### 4. 插件机制应一步到位做成通用框架

今天最重要的架构判断之一，是对插件方向的重新收敛：

- 不建议只做 `channel plugin`
- 应直接设计成通用插件框架
- 第一批 capability bucket 建议覆盖 `channel`、`provider`、`tool`、`service`

这个结论来自对 `.workspace/openclaw` 的参考。其意义在于，`agent-diva` 不应为 channel、provider、tool 分别发明不同扩展体系，否则后面会形成多套平行机制，维护成本持续上升。

### 5. ClawHub 更适合先做“分发层接入”，不是重写技能系统

调研结论很明确：`agent-diva` 并不缺技能运行时，缺的是公共技能分发闭环。

推荐路线是：

- 保留现有 `skills/clawhub/SKILL.md` 作为 agent 侧补充入口
- 首期由 `agent-diva-manager` 封装 `npx --yes clawhub@latest`
- GUI/CLI 基于 manager 提供搜索、安装、更新、状态接口

这比直接用 Rust 重写 registry client 更符合当前阶段，也能更快把技能系统从“可导入”推进到“可搜索、可安装、可立即使用”。

### 6. Onboarding 是正确的 P2，但不应抢占 P0/P1

`onboarding wizard` 的结论不是“不重要”，而是：

- 它有明显用户价值
- 复用现有能力多
- 工程风险低

但它建立在 provider 与配置路径基本闭环的前提上。换句话说，onboarding 更适合在底层登录/发现能力可用后做增强，而不是先用一个更漂亮的向导去包装尚未闭环的 provider 登录问题。

## 主题之间的关系

今天的 5 份文档不是并列的，它们之间有明显依赖链。

### 主链路

1. `provider login` 补齐真实认证闭环
2. `channels login` 提炼统一登录抽象
3. 在统一扩展方向上设计通用插件框架
4. 在 manager / GUI / CLI 层补齐 ClawHub 分发入口
5. 最后把这些能力整合进更完整的 onboarding wizard

### 为什么是这个顺序

- 如果先做 onboarding，容易把“流程更顺”建立在“能力仍是占位”的基础上。
- 如果先做 channel plugin，而不做通用插件框架，后续大概率还要为 provider/tool/service 重做一遍扩展机制。
- 如果先做 Rust 原生 ClawHub client，会把首期目标从“打通公共技能安装闭环”升级成“自建生态协议”，投入不成比例。

## 对今天研究成果的综合判断

### 已经回答清楚的问题

- 与 nanobot 的主要差距到底在哪一层
- `provider login` 为什么是最优先修复项
- `qwen` 为什么不应混入本轮 OAuth 登录
- 插件机制为什么不该只做 channel
- ClawHub 为什么应优先走 manager 封装 CLI 的产品接入方案
- P2 为什么更适合做 onboarding，而不是渠道精细交互

### 仍待后续实现阶段回答的问题

- provider OAuth 的具体协议、回调模式和 token store 接口落在哪个 crate
- channel login trait 的最终接口边界
- 通用插件框架首期采用何种宿主协议与安全边界
- manager 封装 ClawHub CLI 时的 DTO、错误模型和依赖检查细节
- onboarding wizard 的最终交互形态是否需要支持 step 回退与未保存提示

## 推荐执行顺序

### P0

- 补齐 `openai-codex` 的 `provider login` 最小真实闭环
- 同时收敛文档措辞，消除“命令存在但不可用”的产品断层
- 提炼 provider auth metadata 与配置外凭据存储接口

### P1

- 提炼统一 `channels login` 机制
- 启动通用插件框架设计，首批开放 `channel` / `provider` / `tool` / `service`
- 明确插件发现、注册表分桶和安全边界

### P1.5

- 在 `agent-diva-manager` 中封装 ClawHub CLI
- 对外提供 registry status/search/install/update API
- 视资源情况补 CLI 入口，并在 GUI 技能面板提供公共技能搜索与安装

### P2

- 在现有 `run_onboard` 基础上重构为分步 wizard
- 聚焦 provider 导向配置、模型候选增强、summary/确认保存
- 不把 P2 扩展成通用配置引擎或多 section 全量编辑器

## Kanban（nanobot-sync 执行看板）

将上文「主链路」与 P0–P2 拆成可跟踪卡片。**列含义**：完成 = 已在当前仓库形成可用闭环；就绪 = 依赖已满足、可排入迭代；待办 = 未启动或仅方案级。实现推进时请同步更新本表，避免文档与行为再次错位。

### 完成 (Done)

- **P0 · openai-codex `provider login` 真实闭环**（对应 `provider-login-delivery-plan`、`provider-phase1-checklist`）  
  - 含：`agent-diva-core` 外置 auth store / profile、`providers` metadata（`auth_mode` / `login_supported` / `credential_store` / `runtime_backend`）、`openai-codex` 登录 handler、CLI `login/status/logout/use/refresh`、`OpenAiCodex` runtime 消费 token。
- **P0 · provider 认证与配置解耦**  
  - OAuth token 不进入 `config.json`；`provider status` 可展示认证相关维度（实现以 CLI/GUI 为准）。
- **（超出原 Phase1「不做 GUI」范围）GUI 侧 Codex 登录/状态**  
  - `agent-diva-gui` Tauri 已暴露 provider auth 相关命令时可归此类；若产品决定仍算「增量」，可改移到「就绪」。

### 就绪 (Ready) — 建议下一迭代优先拉取

- **P0 收尾 · 用户文档与 CLI 行为一致**  
  - 核对 `docs/user-guide`、`docs/userguide`、外部 docs 站点：不再暗示 `provider login` 为占位；补「手工 smoke」路径说明（若自动化无法跑真 OAuth）。
- **P0 收尾 · `provider login` 测试与契约**  
  - CLI JSON 输出、不支持 login 的 provider 报错语义；可选 fake handler / 集成测试（见 `provider-phase1-implementation-checklist`）。
- **P0-2 · 统一 `channels login` 抽象**（对应 `nanobot-gap-analysis`）  
  - `agent-diva-channels` trait 层定义交互登录能力；CLI 只做路由；至少将现有 WhatsApp 迁入统一机制。
- **P1.5 · ClawHub 产品接入（方案 B）**（对应 `clawhub-registry-integration-plan`）  
  - `agent-diva-manager` 封装 `npx --yes clawhub@latest`；DTO/错误模型/Node 依赖检测；再挂 GUI/CLI。

### 进行中 (Doing)

- （当前迭代正在做的卡片写在这里；无则留空或填「—」。）

### 待办 (Backlog)

- **P1 · 通用插件框架**（对应 `plugin-architecture-reassessment`）  
  - 设计 + 最小原型：统一发现/注册，首批 bucket：`channel` / `provider` / `tool` / `service`；明确安全边界与生命周期。
- **P1 · 通道补齐：WeCom、Mochat**（对应 `nanobot-gap-analysis`）。
- **P1 · Provider 覆盖面：Azure OpenAI、VolcEngine 等**（与 nanobot 文档对齐；`openai-codex` 已单独推进）。
- **P2-A · CLI onboarding wizard**（对应 `onboarding-wizard-p2-assessment`）  
  - 抽 `onboard_wizard` 模块；分步流程；summary + 确认保存/返回；`openai-codex` 在向导中的引导（OAuth vs API key）与现有一致。
- **P2（GUI）· Welcome 分步向导打磨**  
  - 与 CLI 策略对齐：步骤回退、未保存提示、与 `provider login` 文案一致。
- **P2-B · 渠道精细交互**（对应 gap / onboarding 文档中的 P2-B）  
  - 例：Telegram reply context、Feishu reply context、Slack done reaction 等，按渠道逐项排期。
- **多模态 / 统一附件与上下文**（`nanobot-gap-analysis` 后半）  
  - 与 nanobot 对齐的输入侧抽象（图片进上下文、工具链读图等）；独立大块，勿与 P0 混排。
- **第二阶段能力（研究已点名、非首期）**  
  - 第二个 OAuth provider（如 github-copilot）、系统 keychain、Rust 原生 ClawHub client、细粒度 capability matrix 等。

### 卡片与源文档对照

| 卡片主题 | 主要参考文档 |
|----------|----------------|
| Provider OAuth / Phase1 | 同目录 `2026-03-26-provider-login-delivery-plan.md`、`2026-03-26-provider-phase1-implementation-checklist.md`、`2026-03-26-provider-parity-map-from-zeroclaw.md` |
| 差距总览 / channels / 通道 / 多模态 | `2026-03-26-nanobot-gap-analysis.md` |
| 插件 | `2026-03-26-plugin-architecture-reassessment.md` |
| ClawHub | `2026-03-26-clawhub-registry-integration-plan.md` |
| Onboarding | `2026-03-26-onboarding-wizard-p2-assessment.md` |

## 风险提醒

- 如果继续让文档先于实现扩张，用户对 CLI 能力的认知会持续偏离现实。
- 如果先做单点特例而不沉淀抽象，后续 provider/channel/plugin 会出现重复设计。
- 如果过早追求“大而全”，今天已经收敛出的高优先级闭环会被再次稀释。

## 总结

今天 `docs/dev` 的研究成果是高质量且相互支撑的。它没有把结论推向更多“可能性”，而是成功把方向收敛到几条明确主线：

- 先修产品闭环，再做体验包装
- 先建统一扩展框架，再补单一插件类型
- 先接入现有生态工具，再评估是否重写底层客户端

如果按这个研究结论继续推进，`agent-diva` 接下来的开发重心应从“补功能名单”转向“补用户与扩展者真正能走通的路径”。
