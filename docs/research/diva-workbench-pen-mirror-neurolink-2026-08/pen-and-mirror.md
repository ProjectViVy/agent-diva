# PEN 与 Mirror 模块模型

## 1. PEN 的正式候选定义

> PEN 是一个可安装、可授权、可启动、可撤销、可审计的外部能力单元。它把外部运行时
> 或实体的能力投影为 DIVA 可治理的原生能力，但不要求底层使用某一种协议。

PEN 不是单一 wire protocol。一个 PEN 可以通过 stdio、HTTP、WebSocket、MCP、A2A、
厂商 SDK 或未来的 WASM Component 驱动与 Host 通讯。

## 2. 身份模型

至少区分：

| 身份 | 示例 |
| --- | --- |
| `PenPackageId` | `diva.browser.chromium` |
| `PenInstanceId` | 用户电脑上安装的 Chromium PEN 实例 |
| `ExternalEntityId` | 某个浏览器 profile、QQ 账号、电脑或远程 Agent |
| `PrincipalId` | 授予权限的 Owner/Agent/Session 主体 |
| `CapabilityId` | `browser.navigate`、`qq.send_message` |
| `ConnectionId` | 某次实际运行连接 |

包身份、实例身份和外部实体身份不能混为一体。更新软件包不应改变 QQ 账号身份；同一
软件包的两个实例也不能共享审计主体。

## 3. PEN Manifest 最小字段

- ID、版本、发布者、内容摘要和 DIVA 兼容范围；
- runtime kind、入口点和支持的 transport；
- exports：Tool、Channel、Agent、Resource、Event、UI；
- imports：需要从 DIVA 获得的受限能力；
- 网络域名、文件目录、屏幕输入、上传下载等权限；
- Secret 声明，仅引用 Secret Broker 句柄；
- 健康检查、重启策略、资源限制和超时；
- 配置与 UI Schema；
- 签名、证明、更新渠道和回滚信息。

权限空集合必须表示“没有权限”，不能像部分插件系统那样解释为“全部允许”。

## 4. PEN 导出与原生 Tool

推荐投影接口：

~~~text
PenRuntime
├─ install / configure
├─ start / stop / restart
├─ health / logs
└─ uninstall

PenConnection
├─ handshake
├─ call
├─ subscribe
├─ cancel
└─ close

CapabilityProjection
├─ ToolExport
├─ ChannelExport
├─ AgentExport
├─ ResourceExport
├─ EventExport
└─ UiExport
~~~

Agent 调用的是 ToolRegistry 中的原生 Tool；Tool 实现把请求转给 PEN。这样可以让
Tool schema、审批、审计和上下文装配保持 DIVA-native，同时把实现彻底外接。

## 5. MCP、Channel 和 A2A 的迁移关系

推荐“迁移到共同地基，不做语义清零”：

- MCP Server 可以被包装为一种 PEN Driver，MCP Tool 投影为原生 Tool；
- MCP Resources/Prompts 以后映射到相应投影，不能永久丢弃；
- QQ 可以成为 ChannelExport 的安全 PEN，但仍保留 sender/chat/thread/delivery 语义；
- 远程 A2A Agent 可以映射为 External Entity + AgentExport；
- Manager 原生 A2A Server 不应仅为了存在而依赖 PEN；
- 现有内建 Channel/MCP 与新模块长期并存，按 TCK 逐个迁移，不进行 flag-day rewrite。

## 6. PEN 样板

### Browser PEN

- 独立浏览器进程和 profile；
- 域名、下载、上传、凭据和支付权限；
- 导出 navigate、read、click、type、screenshot 等 Tool；
- 导出下载、弹窗、页面变化等 Event；
- 高风险表单提交、购买和凭据使用进入 Approval。

### Secure QQ PEN

- QQ 凭据和 SDK 隔离在独立进程；
- 导出 typed Channel envelope，而不是任意 metadata；
- 私聊、群聊、Guild 使用不同信任和记忆策略；
- token、网络、文件和媒体权限最小化；
- sender 自报信息不得作为认证。

### Computer Control PEN

- 最高权限等级；
- 限定屏幕、窗口、键鼠和文件范围；
- 前台存在、动作审批、全局 Kill Switch；
- 可选会话记录，但必须支持敏感区域遮挡和审计脱敏。

## 7. Mirror 的正式候选定义

> Mirror 是 DIVA 的可替换呈现与具身前端。它消费 DIVA 的语义状态并产生受限用户输入，
> 但默认不获得通用外部执行权。

Mirror 可以是：

- VRM/Live2D/2D 桌宠；
- 纯语音形态；
- 终端、手机、网页、AR 或硬件灯光；
- Workbench 内嵌舞台；
- 独立桌面 Overlay。

## 8. Mirror 合同

建议拆为：

- `PresentationCapabilities`：文字、字幕、音频、动作、表情、通知；
- `InputCapabilities`：语音、触摸、摄像头请求、手势；
- `PresenceState`：online、idle、thinking、speaking、working、approval；
- `AppearanceDescriptor`：形象、资源、语言和无障碍能力；
- `MirrorHealth`：渲染、音频、输入设备和资源状态。

Mirror 接收语义表达提示，不接收硬编码动画文件名。动作映射由 Mirror 自己决定。

## 9. 当前 Mate 的迁移方向

现有 Mate 已具备内嵌视图、独立 Overlay、VRM、语音输入、TTS、字幕和模型管理，是首个
Mirror 样板。迁移应分阶段：

1. 先冻结 Presentation/Input 事件；
2. 在当前 GUI 内让 Mate 改用新事件，不改变用户体验；
3. 将资源和渲染逻辑收拢成 Mirror 模块；
4. 通过 Neuro-Link 在独立进程运行；
5. 保留第一方默认安装，同时解除 Core/主 GUI 强耦合；
6. 增加第二个极简 Mirror 验证合同没有绑定 VRM。

## 10. 自主性边界

PEN 和 Mirror 都不应在 v1 内自带绕过 DIVA 的自治调度权。需要定时或后台运行时，应
申请现有 Cron、Run、Heartbeat 或受监督任务能力。OpenFang Hand 那样的自治应用包未来
可以运行在模块底座上，但不能让每个模块各自形成第二套 Agent Runtime 和治理系统。
