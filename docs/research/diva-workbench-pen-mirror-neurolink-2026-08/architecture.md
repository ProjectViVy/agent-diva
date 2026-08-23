# 总体架构与 DIVA 瘦身边界

## 1. 目标结构

~~~text
                         外部世界
          ┌────────────────┼────────────────┐
          │                │                │
       A2A Agent         QQ/平台          浏览器/电脑
          │                │                │
          └────── PEN / Channel / A2A ─────┘
                           │
                    ┌──────▼──────┐
                    │  DIVA Core  │
                    │ Agent/Harness│
                    │ Policy/BML  │
                    │ Task/Run    │
                    └──────┬──────┘
                           │
                     Neuro-Link
              身份、会话、状态、表达、控制
                           │
          ┌────────────────┼────────────────┐
          │                │                │
       Workbench GUI    Desktop Mirror   手机/网页/AR前端
~~~

Workbench 是官方参考前端，但 Neuro-Link 允许其他前端完整替代它的用户交互表面。
Core 可以完全无头运行，且不依赖某个具体 Mirror 或 GUI 才能处理任务。

## 2. Core 应长期保留

- AgentLoop 与 Harness；
- Session、Context、Task、Run 和取消；
- Provider 抽象与模型调用；
- 身份、Policy、Approval、Audit；
- Sandbox；
- Laputa 与 BML；
- 统一事件模型；
- Neuro-Link、PEN、Mirror 的最小协议合同和受监督注册入口。

这些能力构成 DIVA 的认知连续性和治理边界，不能因插件化而外包给任意模块。

## 3. 应逐步迁出的重型能力

- VRM/Live2D 渲染、模型和动画资产；
- 桌宠窗口、舞台布局和形象管理；
- TTS 播放、字幕、口型和动作映射；
- 浏览器、OCR、深度视觉和屏幕捕获；
- QQ 等高风险平台客户端；
- 电脑控制和厂商设备 SDK；
- 特定前端的主题、颜色和交互状态机。

迁出不等于删除。第一方实现可以继续随产品发行，但应通过稳定合同运行，而不是成为
Core 或主 GUI 无法移除的编译耦合。

## 4. 语义事件替代 Avatar 特例

Core 应输出与渲染器无关的表达事件，例如：

~~~text
assistant.thinking
assistant.speaking
assistant.waiting_for_approval
assistant.tool_started
assistant.tool_completed
persona.expression_hint
conversation.delta
conversation.completed
~~~

Mirror 决定 `assistant.thinking` 显示为 VRM 思考动作、呼吸灯、终端状态文字还是纯语音
停顿。Core 不知道具体 VRM 动画名，也不通过特殊 chat ID 指挥形象。

## 5. 能力投影层

外部模块连接后，先经过身份、权限和兼容性检查，再投影到现有运行时：

~~~text
Module Package / Instance
        ↓
Manifest + Trust + Policy
        ↓
Host / Supervisor / Connection
        ↓
Capability Projection
├─ ToolExport     → ToolRegistry
├─ ChannelExport  → Channel / MessageBus
├─ AgentExport    → A2A Adapter / Router
├─ ResourceExport → Context Resource
├─ EventExport    → Event Bus
└─ UiExport       → Workbench Panel
~~~

同一个 PEN 可以导出多个能力；同一种能力也可以有多个后端。Workbench 不应把“一个
桌面图标等于一个 Tool”作为永久数据模型，而应展示 Package、Instance、External Entity
和 Capability 的真实拓扑。

## 6. 运行时与传输

MVP 建议支持：

1. 受监督独立进程 + stdio；
2. 受认证 loopback WebSocket/HTTP；
3. 显式配置的远程 HTTPS 服务。

后续可以增加 WASM Component、容器或移动设备专用运行时。第三方模块默认进程外运行，
不建立“安装任意代码后直接导入 Manager 进程”的捷径。

## 7. 热插拔一致性

模块安装、移除和权限变化可以热生效，但 Agent 运行必须具有确定性：

- 每个 turn/run 开始时生成 Capability Snapshot；
- 运行中不静默改变模型看到的工具定义；
- 权限撤销立即阻止新调用，并取消或隔离在途调用；
- 下一 turn 才使用新的能力集合；
- 所有能力快照带版本和审计关联。

这可以复用当前 ToolRegistry 和按 turn 重建工具集合的基础，同时避免中途换工具导致
审计、重试和上下文缓存失真。
