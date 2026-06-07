# ClawHub 公共技能注册表接入方案评估

## 背景

`agent-diva` 已经具备技能体系与本地技能管理能力：

- Agent 运行时会从工作区 `~/.agent-diva/workspace/skills/` 与内置 `skills/` 目录加载 `SKILL.md`
- `agent-diva-manager` 已支持技能列表、ZIP 上传、删除
- GUI 设置页已经有技能管理面板
- 仓库内已经存在内置技能 `skills/clawhub/SKILL.md`

这意味着项目并不是“没有 ClawHub”，而是目前只有一层“让 agent 自己会调用 `npx clawhub`”的技能说明，尚未形成产品级“搜索 -> 安装 -> 立即可用”的公共技能注册表闭环。

参考对象：

- `.workspace/nanobot/nanobot/skills/clawhub/SKILL.md`
- `.workspace/nanobot/README.md`
- `agent-diva-agent/src/skills.rs`
- `agent-diva-manager/src/skill_service.rs`
- `agent-diva-gui/src/components/settings/SkillManagementCard.vue`

## 现状判断

### 已有能力

`agent-diva` 当前已经具备以下基础条件：

1. 技能加载闭环已经存在，且工作区技能优先级高于内置技能。
2. 技能目录约定已经稳定，ClawHub 安装目标目录与现有技能加载路径天然兼容。
3. GUI 和 manager 已经有技能管理入口，不需要再新造一套“插件市场”基础设施。

### 当前缺口

当前缺的不是技能运行时，而是“分发层接入”：

1. 用户无法在产品内搜索公共技能。
2. 用户无法通过 CLI/GUI 直接从注册表安装技能。
3. 安装后的依赖检查、失败提示、重新开会话提示，没有形成统一产品体验。
4. 现有 `skills/clawhub/SKILL.md` 更像给 agent 用的“操作手册”，不是给最终用户的“产品入口”。

### 对“装完即用”定位的意义

ClawHub 这类公共技能注册表对 `agent-diva` 的价值，不在于增加一个新能力类别，而在于压缩用户首次完成扩展的路径：

- 当前路径：用户先知道技能概念，再理解目录结构，再准备 ZIP 或手动复制目录，最后重开会话。
- 接入后路径：搜索、安装、提示依赖、开始新会话。

这更符合 `agent-diva` 当前“默认本地网关 + GUI 可视化管理 + 安装后可用”的产品方向。

## 实现方式评估

### 方案 A：仅保留当前内置 `clawhub` skill

做法：

- 不新增 manager / GUI / CLI 产品能力
- 继续依赖 agent 在对话中自主调用 `npx --yes clawhub@latest ...`

优点：

- 零新增后端接口
- 技术成本最低

缺点：

- 对最终用户不可见
- 是否会触发、何时触发、如何处理失败，完全依赖 agent 推理
- 难以形成稳定的“技能市场”体验
- 不符合“装完即用”的产品目标

结论：

不建议作为主方案。它可以保留，但只能作为补充入口。

### 方案 B：通过 manager 封装 ClawHub CLI，作为产品级接入层

做法：

- 在 `agent-diva-manager` 中新增 ClawHub registry service
- 由 manager 统一调用 `npx --yes clawhub@latest`
- 固定使用 `--workdir <workspace>`
- GUI / CLI 通过 manager 提供的接口执行搜索、安装、更新、列出已安装来源

优点：

- 复用 ClawHub 现成分发能力，避免首期自建 registry client
- 与现有工作区技能加载路径完全兼容
- 改动集中在 manager / GUI / CLI，`agent-diva-agent` 基本无需修改
- 可以统一处理 Node.js / `npx` 缺失、网络失败、安装后提示重开会话等用户体验问题

缺点：

- 引入 Node.js / `npx` 作为运行时依赖
- 需要管理外部命令执行、安全边界和输出解析
- 首次安装依赖网络与 npm 生态稳定性

结论：

这是最适合 `agent-diva` 当前阶段的主方案，也是推荐的 Phase 1。

### 方案 C：在 Rust 中直接实现原生 ClawHub registry client

做法：

- 直接调用 ClawHub registry API 或协议
- 在 Rust 侧下载、校验、解压、安装技能包

优点：

- 不依赖 Node.js
- 输出结构、错误模型、缓存策略完全可控
- 更容易做离线缓存、签名校验、版本锁定

缺点：

- 前提是 registry API/协议稳定且文档清晰
- 首期成本明显更高
- 容易过早把精力投入到“重写已有生态工具”

结论：

适合作为 Phase 2/3 演进方向，不建议作为首期落地方式。

## 推荐方案

推荐采用“方案 B 为主，方案 A 保留，方案 C 预留”的分层策略：

1. 保留内置 `skills/clawhub/SKILL.md`
2. 新增产品级 ClawHub 接入能力
3. 首期通过 manager 封装 ClawHub CLI
4. 后续若 registry 协议稳定，再评估 Rust 原生 client

核心原因：

- 现有技能系统已经闭环，安装目标目录也已稳定
- manager 本身已经负责技能上传/删除，天然适合继续承担“技能安装器”角色
- GUI 已有技能管理卡片，只需扩展为“本地技能 + 公共注册表”双入口

## 推荐分层设计

### 1. `agent-diva-agent`

原则：尽量不改。

原因：

- Skill Loader 已经基于工作区目录加载技能
- 只要安装结果落到 `~/.agent-diva/workspace/skills/<skill>/`，agent 运行时就能识别

首期只需要继续保留：

- 技能可用性检查
- 新会话后重新加载技能摘要

### 2. `agent-diva-manager`

这是首期主改动层。

建议新增一个独立服务，例如：

- `clawhub_service.rs`

职责：

1. 检查 `node` / `npx` 是否存在
2. 统一拼装 `clawhub` 命令
3. 固定传入工作区目录
4. 规范化 search/install/update/list 的结果
5. 将外部命令错误转换成可读的 API 错误

建议新增 API：

- `GET /api/skills/registry/status`
- `POST /api/skills/registry/search`
- `POST /api/skills/registry/install`
- `POST /api/skills/registry/update`

其中：

- `status` 用于检查 Node.js / `npx` 是否可用，以及注册表入口是否可启用
- `search` 返回公共技能搜索结果
- `install` 负责安装指定 slug 到工作区
- `update` 负责更新已安装的 registry 技能

不建议首期把“注册表技能列表”和“本地技能列表”混成一个接口。更清晰的做法是：

- 本地已安装技能继续走现有 `/api/skills`
- 公共注册表能力走 `/api/skills/registry/*`

### 3. `agent-diva-cli`

建议补一组显式命令，而不是只依赖 GUI：

- `agent-diva skills search <query>`
- `agent-diva skills install <slug>`
- `agent-diva skills update [--all|<slug>]`
- `agent-diva skills registry-status`

原因：

- 这比“让用户在对话里召唤 clawhub skill”更稳定
- 也方便远程环境、无 GUI 场景
- CLI 可以复用 manager 的同一套错误与结果模型

如果当前不希望扩展 CLI 面，可先只做 GUI + manager；但从工程一致性看，CLI 最终仍应补齐。

### 4. `agent-diva-gui`

当前技能设置卡片已经支持：

- 刷新
- ZIP 上传
- 删除

建议扩展为两个区块：

1. 已安装技能
2. ClawHub 公共技能搜索与安装

最小交互建议：

- 输入搜索词
- 展示搜索结果卡片
- 点击安装
- 安装成功后自动刷新本地技能列表
- 显示“新技能将在新会话中可用”的提示

这能把现有技能面板从“导入本地 ZIP”升级为“技能分发中心”。

## 关键实现细节

### 1. 工作区路径必须由 manager 固定注入

这一点应直接固化在 manager，而不是交给前端或 agent 拼命令。

原因：

- `skills/clawhub/SKILL.md` 已经明确 `--workdir ~/.agent-diva/workspace` 是关键参数
- 如果由前端或用户传入，容易出现安装到当前目录、错误目录或不一致目录

因此 manager 应始终基于当前配置解析出 workspace，然后附加：

- `--workdir <resolved_workspace>`

### 2. 外部命令执行边界

需要限制 manager 调用方式，避免把 registry 接口做成通用 shell 执行器。

建议约束：

- 命令固定为 `npx --yes clawhub@latest`
- 子命令仅允许 `search` / `install` / `update` / `list`
- `slug` 与查询参数做基础校验
- 不允许任意附加参数透传

### 3. 输出模型不要直接绑定 CLI 文本

由于外部 CLI 输出格式可能演进，manager 层应尽量做一层适配，向 GUI / CLI 暴露稳定 DTO，而不是直接透传原始文本。

建议 DTO 至少包含：

- 搜索结果：`slug`、`name`、`description`、`homepage`、`version`
- 安装结果：`slug`、`installed_path`、`replaced_existing`、`requires_restart`
- 运行状态：`node_available`、`npx_available`、`message`

如果 ClawHub CLI 当前没有稳定机器可读输出，首期可以：

1. 先把返回值收敛成“标准化文本 + 成败状态”
2. GUI 初版使用简单列表
3. 等确认其输出可稳定解析后，再升级为更结构化 DTO

### 4. “装完即用”的真实边界

严格说，当前技能体系不是“热加载即用”，而是“安装后新会话可用”。

因此文案和验收都应诚实表达：

- 安装后自动出现在技能列表
- 新开会话后 agent 可读到新技能

首期不建议为此强行增加运行中 Agent 的热更新机制，因为那会把问题从“分发层接入”扩大到“对话上下文实时重建”。

### 5. 依赖与可用性提示

ClawHub 方案的一个现实约束是 Node.js。

首期至少要显式处理三类情况：

1. 本机未安装 `node` / `npx`
2. 网络不可达，无法下载 `clawhub@latest`
3. 技能已安装，但依赖要求不满足，导致在技能列表中显示 unavailable

第三类场景尤其需要串起来，因为 `agent-diva` 当前已经有技能可用性检查能力。安装成功不等于可立即使用，GUI 应在刷新列表后复用现有 `available` 状态提示。

## 分阶段建议

### Phase 0：文档和定位对齐

目标：

- 把 ClawHub 明确为“公共技能注册表接入”，不是“再做一个技能系统”
- 保持内置 `clawhub` skill 作为 agent 自助入口

产物：

- 本文档

### Phase 1：manager + GUI 最小闭环

目标：

- 在设置页内搜索并安装公共技能

范围：

- `agent-diva-manager`
- `agent-diva-gui`

最小验收：

1. GUI 能检测 registry 是否可用
2. GUI 能搜索技能
3. GUI 能安装 skill 到工作区
4. 安装后本地技能列表自动刷新
5. 用户能看到“新会话生效”的提示

### Phase 2：CLI 补齐

目标：

- 无 GUI 场景也能直接使用公共技能注册表

范围：

- `agent-diva-cli`

最小验收：

1. `agent-diva skills search <query>` 可用
2. `agent-diva skills install <slug>` 可用
3. 错误输出与 GUI/manager 保持一致

### Phase 3：原生化与增强

可选方向：

- Rust 原生 registry client
- 版本固定与升级策略
- 技能来源标记与 provenance 展示
- 签名/校验/信任策略
- 已安装 registry 技能与手动 ZIP 技能的来源区分

## 风险与边界

### 风险 1：Node.js 依赖提高门槛

缓解方式：

- 在 GUI 中先检查 registry status，再决定是否展示可安装入口
- 缺失依赖时给出明确安装提示

### 风险 2：外部 CLI 输出不稳定

缓解方式：

- manager 做适配层，不把原始输出直接扩散到前端接口
- 首期少承诺复杂元数据，优先保证“能搜、能装、能报错”

### 风险 3：公共技能带来供应链风险

缓解方式：

- 首期至少明确来源为“registry 安装”
- 保留工作区目录隔离
- 后续考虑签名、可信发布者、校验摘要

### 风险 4：用户误解为运行时热加载

缓解方式：

- 安装成功后统一提示“开始新会话后可用”
- 不在首期文档中承诺运行中热更新

## 最终建议

从 `agent-diva` 当前架构出发，最合理的路线不是重写技能系统，也不是直接做 Rust 原生 registry，而是：

1. 继续保留现有 `skills/clawhub/SKILL.md` 作为 agent 自助入口
2. 以 `agent-diva-manager` 为中心封装 ClawHub CLI
3. 优先在 GUI 技能管理页补齐搜索/安装闭环
4. 随后补 CLI 对等入口

这样可以用最小改动，把现有“本地技能导入”升级为“公共技能分发 + 本地加载”的完整产品路径，且不会破坏已经稳定的技能加载架构。
