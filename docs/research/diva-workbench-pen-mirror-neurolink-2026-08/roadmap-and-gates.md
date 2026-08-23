# 安全模型、Epic 路线与研究门禁

## 1. 横切安全原则

- 第三方 PEN/Mirror 默认进程外运行；
- 空授权表示无权限；
- Package、Instance、External Entity 和 Principal 身份分离；
- 权限按 Owner、Agent、Channel、Session、Device 和 Instance 上下文授予；
- 外部输入统一按不可信数据处理；
- 所有网络、文件、屏幕、摄像头、麦克风和位置能力显式声明；
- Secret 通过句柄/租约提供，不默认注入普通环境变量；
- 高风险能力支持审批、取消、超时、预算和 Kill Switch；
- 日志、事件和崩溃报告不得携带原始凭据和连续感知媒体；
- PEN、Mirror、Channel、A2A 均不得直接写 BML；
- 感知形成 Memory 必须经过 Moment/Consent/Laputa 边界。

## 2. 模块信任等级

| 等级 | 示例 | 默认运行方式 |
| --- | --- | --- |
| Built-in Trusted | DIVA 核心和受审查第一方模块 | 可编译内置或进程外 |
| Signed First-party | Browser PEN、默认 Mirror | 进程外，自动更新可配置 |
| Signed Third-party | 厂商设备、社区 Mirror | 进程外，显式授权 |
| Local Development | 开发中的 PEN | debug 模式、强提示、有限权限 |
| Untrusted/Unsigned | 未知包 | 默认拒绝运行 |

签名有效只证明内容与某个密钥一致；Trust Store 仍需决定该发布者是否可信。

## 3. 各高风险场景

### Neuro-Link 前端

- 设备注册、撤销和会话令牌；
- loopback 也认证；
- 工作区能力分级；
- 审批不能被不支持安全 UI 的前端静默代答；
- 重连不能重复危险操作。

### Browser PEN

- 域名 allowlist；
- 上传、下载、剪贴板和凭据独立权限；
- 支付、提交和外发进入审批；
- 浏览器返回内容按 prompt injection/不可信输入处理。

### QQ/平台 PEN

- 凭据和平台 SDK 隔离；
- 主人私聊、陌生人和群聊使用不同信任层；
- 群聊内容不得直接进入 Memory；
- typed envelope、去重、限速、重连和媒体大小限制。

### Computer Control PEN

- 最高风险分级；
- 明确前台可见和硬件/软件 Kill Switch；
- 屏幕区域、窗口、键鼠和文件范围限制；
- 敏感应用/字段遮挡；
- 动作级审计和必要审批。

### Companion Node

- 录制指示和物理关闭优先于软件状态；
- 原始媒体短期、加密、有界；
- 旁观者隐私；
- 位置默认粗粒度；
- 离线缓存可查看和清理；
- Observation 默认不进入 Memory。

## 4. 顶层 Epic

### EPIC A：WORKBENCH-SHELL-AND-MODULE-MODEL

- 冻结 WorkbenchModule、Package、Instance、Manifest 和 Trust；
- 建立 Catalog、安装、配置、更新和 Supervisor；
- Workbench 组合现有工作区，但不改变领域权威；
- Echo Module 和只读 Catalog 作为首个 TCK。

### EPIC B：NEURO-LINK-FRONTEND-FABRIC

- 冻结前端身份、能力协商和多路复用 envelope；
- Conversation/Presentation/Control/State Sync；
- bounded queue、ACK、取消、重连和快照；
- 当前 GUI 或最小 Web Client 成为首个客户端；
- Mate speak 特例迁移为 Presentation Event。

### EPIC C：PEN-EXTERNAL-CAPABILITY-SYSTEM

- PEN Host、Manifest、Capability Projection；
- ToolRegistry 投影和 Capability Snapshot；
- MCP Driver；
- Browser PEN；
- Secure QQ PEN、Computer Control PEN 分别作为后续高风险切片。

### EPIC D：MIRROR-EMBODIMENT-SYSTEM

- 冻结 Mirror Presentation/Input 合同；
- Mate 先在原位改用语义事件；
- 再迁为第一方 Mirror Module；
- 极简文字/语音 Mirror 验证合同不绑定 VRM；
- Workbench 提供 Mirror Stage、资源和健康状态。

### EPIC E：COMPANION-NODE-AND-EXPERIENCE

- 手机作为第一台 Companion Node；
- Camera/Microphone/Location Sensor PEN；
- Local Guardian 和离线缓存；
- Observation/Moment/Memory 边界；
- Experience Store/Consent Receipt 研究；
- 生活场景 E2E 后再评估专用硬件。

## 5. 推荐依赖顺序

~~~text
R0 词汇、领域权威、威胁模型
  ↓
R1 共同 Module Package + Semantic Event
  ├─ R2 Neuro-Link 最小前端链路
  │    └─ R3 Mate → Mirror 样板
  ├─ R2 PEN Tool Projection
  │    └─ R3 Browser PEN
  └─ R2 Workbench 只读 Catalog/Topology
       ↓
R4 Secure QQ PEN + Channel TCK
       ↓
R5 手机 Companion Node + Experience/Moment
       ↓
R6 A2A/既有 MCP/Channel 渐进迁移
~~~

Browser PEN 比 QQ 更适合作为第一条基础设施纵向链路，因为它能同时验证生命周期、Tool、
事件、流和权限，而不立即承担群聊信任和平台凭据的全部复杂度。QQ PEN 应作为第二条
Channel 语义与隔离证明。

## 6. 正式施工前必须拍板

1. 是否接受 PEN Package、Instance、External Entity 三层身份；
2. v1 第三方模块是否一律进程外；
3. MVP 是否只做 process/stdio + authenticated loopback，WASM 延后；
4. WorkbenchModule 是否作为中性部署类型，PEN/Mirror/Panel/Adapter 为兄弟 kind；
5. Neuro-Link 是否采用实时流 + Versioned Service Binding，而非复制所有业务 API；
6. 是否以当前 GUI/最小 Web Client 作为 Neuro-Link 首个 TCK；
7. 是否以 Mate 为首个 Mirror、Browser 为首个 PEN；
8. Experience Store 是否立项为独立短期权威，而不是 BML 表；
9. Companion Node 第一阶段是否明确只做手机，不启动硬件项目；
10. 现有 MCP/Channel 是否接受长期共存和逐个适配，而非一次性迁移。

## 7. 验收门禁

每条实现 Epic 至少具备：

- ADR/合同和威胁模型；
- 离线协议 fixture；
- 进程崩溃、重启、超时、取消和权限撤销测试；
- 有界队列和背压测试；
- 签名/身份/授权失败负向测试；
- Secret 和日志脱敏检查；
- 不直写 BML/Laputa 边界检查；
- 真实前端或真实能力的最小纵向 smoke；
- 对应 docs/logs 四件套；
- 不提高 Rust 1.80 MSRV，除非另行批准。

## 8. 明确不做

- 不把 Workbench 做成第二套 Manager；
- 不把 Mirror 赋予默认通用 Tool 权限；
- 不把 Neuro-Link 降格为 PEN WebSocket；
- 不让每个模块自建 Agent Runtime、Cron 和 Memory；
- 不在首版支持任意未签名宿主内插件；
- 不以连续录音录像作为伴生体验默认值；
- 不让研究文档替代正式设计评审和实现授权。
