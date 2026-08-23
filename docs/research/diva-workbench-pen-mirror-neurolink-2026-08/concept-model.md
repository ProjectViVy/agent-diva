# 概念模型与边界

## 1. 需要解决的核心问题

当前 DIVA 同时面对四种扩张：

1. A2A、QQ、浏览器、电脑控制等外部实体接入；
2. 第三方前端、移动端和未来专用设备接入；
3. 桌宠、语音、VRM、Live2D 等具身呈现；
4. OCR、深度视觉、浏览器和复杂工作流进入工作台。

如果继续把它们都理解为 Skill、Tool 或普通 Channel，最终会产生一个能力臃肿、权限
含混、前后端紧耦合的主程序。新概念模型必须同时回答：

- 能力在哪里运行；
- 谁拥有权限；
- 谁是用户交互前端；
- 什么只负责呈现；
- 什么可以产生外部动作；
- 什么可以写入持久化权威。

## 2. 术语表

| 概念 | 本质 | 独立运行时 | 主要权力 |
| --- | --- | ---: | --- |
| Skill | 提示词、知识和操作流程 | 通常没有 | 不因存在而获得执行权 |
| Tool | 模型可调用的操作界面 | 不一定 | 受当前 Session/Policy 授权 |
| Channel | 平台会话的异步入站/出站适配 | 通常有 | 消息投递，不自动获得核心权威 |
| MCP | Tools/Resources/Prompts 等标准协议 | Server 可独立 | 由客户端和策略决定 |
| A2A | 远程 Agent 发现与 Task 协议 | 通常远程 | 受 Agent Policy、预算和审批约束 |
| Neuro-Link | 完整前端接入面 | Client 独立 | 高信任用户交互与控制 |
| PEN | 外部能力单元 | 默认独立 | Manifest 请求并经策略授予 |
| Mirror | 具身与呈现单元 | 可独立 | 默认只有呈现和受限输入权 |
| Workbench | 第一方模块化前端和控制面 | 是 | 组合服务，不成为领域权威 |
| Companion Node | 随身前端/传感/具身节点 | 是 | 本地隐私边界和设备能力 |

## 3. PEN 与 Skill

Skill 回答“Agent 应该怎样做”；PEN 回答“某个外部能力怎样被安装、启动、授权、调用、
撤销和审计”。PEN 可以导出原生 Tool，但不等于 Tool；一个 PEN 还可以导出 Channel、
Agent、Resource、Event 或配置 UI。

示例：

- `browser-research/SKILL.md` 是浏览器使用流程；
- Browser PEN 是独立浏览器进程、会话、域名权限、下载权限和原生工具投影；
- Skill 可以调用 Browser PEN，但不能取代它的隔离与权限模型。

## 4. PEN 与 Mirror

PEN 和 Mirror 是兄弟类型，而不是父子类型：

~~~text
PEN:
DIVA ──命令──> 外部能力
DIVA <──结果── 外部能力

Mirror:
DIVA ──状态/表达──> 用户
DIVA <──交互输入──── 用户
~~~

Mirror 默认不拥有通用外部执行权。桌宠表达“正在打开网页”不意味着桌宠进程可以直接
操作浏览器；真正的执行应交给 Browser PEN。Mirror 可以申请 Sensor/Input 能力，但仍
经过设备权限和 DIVA Policy。

## 5. Neuro-Link 与普通 Channel

普通 Channel 面向 Telegram、QQ、Discord 等平台，核心语义是 sender、chat、thread、
message、delivery 和平台能力。

Neuro-Link 面向 DIVA 的 Owner Frontend，除了 Conversation，还需要：

- Presentation：语音、字幕、动作、表情、通知；
- Control：审批、暂停、恢复、停止、切换 Session；
- Workspace：各领域服务的发现和绑定；
- State Sync：快照、事件游标、断线恢复、冲突处理；
- Device：设备身份、前端能力和可信输入指示。

因此 Neuro-Link 可以保留“重量级 Channel”的产品定位，但生产合同不应继续受当前
`ChannelHandler` 最小接口限制。

## 6. Workbench 的权威边界

Workbench 是模块化产品壳，不是新的万能后端：

~~~text
Workbench
├─ Core Workspaces
│  ├─ Chat
│  ├─ Persona
│  ├─ Memory
│  ├─ Evolution
│  └─ Planning / Files
├─ PEN Dock
├─ Mirror Stage
└─ System Console
~~~

它可以组合、导航和展示这些领域，但不得：

- 直接写 BML；
- 绕过 Laputa 修改 Persona；
- 将审批中心和内容治理混成一个权威；
- 用前端缓存成为 Session、Task 或 Memory 的第二真相源。

## 7. 共同模块底座

推荐建立只负责部署和生命周期的中性模块抽象：

~~~text
WorkbenchModule
├─ PenModule       外部执行能力
├─ MirrorModule    具身与呈现
├─ PanelModule     工作台 UI
└─ AdapterModule   MCP / A2A / Channel 协议适配
~~~

共同能力包括包 ID、版本、发布者、签名、安装、更新、配置 Schema、Supervisor、健康
检查、日志和 Secret Broker。模块种类决定可导出接口和默认权限，不能使用一个
`execute(any_json)` 万能入口代替类型合同。
