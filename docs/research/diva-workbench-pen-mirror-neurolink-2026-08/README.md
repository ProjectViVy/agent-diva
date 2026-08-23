# DIVA 工作台、PEN、Mirror、Neuro-Link 与伴生节点研究包

> 日期：2026-08-23
> 状态：Research / Proposal；用户已认可总体方向，尚未授权生产实现
> 范围：DIVA 外接能力、完整前端、具身呈现、工作台产品壳，以及随用户共同生活的伴生形态

## 结论先行

DIVA 下一阶段不应继续把重型能力、平台客户端、浏览器和桌宠全部编入主程序。推荐的
长期形态是一个保持稳定的无头 Core，加上四个语义明确、可以独立演进的系统：

- **Neuro-Link**：DIVA 的完整前端接入面和“神经连接”，不是普通社交频道，也不是
  PEN 的专用传输协议；
- **PEN（钢笔）**：拥有独立身份、权限、生命周期和运行边界的外部能力单元；
- **Mirror**：DIVA 面向人的可替换形象、声音、具身与交互前端；
- **Workbench**：组合核心工作区、PEN、Mirror、审批、连接拓扑和系统控制的第一方
  产品壳与模块化运行环境。

面向随用户吃饭、学习和旅行的长期愿景，再增加一种部署形态：

- **Companion Node（伴生节点）**：手机、眼镜、胸针、桌面机器人或专用设备上的
  Neuro-Link 前端、Mirror、Sensor PEN、本地隐私守卫和离线缓存组合。

可以用一个稳定的比喻理解边界：

| 对象 | 比喻 | 负责 |
| --- | --- | --- |
| DIVA Core | 大脑 | Agent/Harness、Session、Task、Policy、Laputa/BML |
| Neuro-Link | 神经 | 前端身份、会话、状态同步、表达与控制 |
| PEN | 手和外部器官 | 浏览器、QQ、电脑控制、传感器、MCP 等能力 |
| Mirror | 脸和身体 | 形象、声音、动作、字幕、存在感和用户输入 |
| Workbench | 桌面与房间 | 组织工作区、能力、具身、审批和系统控制 |

## 研究包导航

1. [概念模型与边界](./concept-model.md)
2. [总体架构与 DIVA 瘦身边界](./architecture.md)
3. [Neuro-Link 完整前端接入面](./neurolink-front-end-fabric.md)
4. [PEN 与 Mirror 模块模型](./pen-and-mirror.md)
5. [伴生节点、生活场景与 Experience 模型](./companion-node.md)
6. [当前实现、参考项目与标准证据](./current-state-and-references.md)
7. [安全模型、Epic 路线与研究门禁](./roadmap-and-gates.md)

## 关键架构判断

### 共享基础设施，但不压平语义

PEN、Mirror、Workbench Panel 和协议 Adapter 可以共享 Manifest、包签名、安装、升级、
Supervisor、Secret Broker、健康检查、日志和兼容性机制；但不能被压成一个万能 trait。

Tool、Channel、MCP、A2A、Neuro-Link 各自保留原本语义：

- Tool 是模型可调用的请求/响应操作；
- 普通 Channel 是人或平台的异步消息入口；
- MCP 是能力发现和调用协议；
- A2A 是远程 Agent 发现、Task、Message、Artifact 和取消协议；
- Neuro-Link 是高信任 Owner/Frontend Interface；
- PEN 是能力的部署、授权与运行边界；
- Mirror 是表达和具身边界；
- Workbench 是产品前端与控制面，不是跨领域数据权威。

### 工作台不吞并现有工作区权威

Workbench 可以组合 Chat、Persona、Memory、Evolution、Planning 和 Files，但不得取代
这些领域各自的 Manager Service、存储和治理合同。现有
[产品范围与工作台边界](../../decisions/product-scope-and-workbench-2026-06-18.md)
明确要求工作台不能重新引入跨领域 Proposal/Governance 页面，本研究延续该约束。

### 感知不等于记忆

生活陪伴必须区分短暂观察、用户选择保留的 Moment 和经治理进入 BML 的长期 Memory。
摄像头、麦克风和位置输入默认只服务当前交互；原始媒体不得因接入 DIVA 就自动沉淀。

## 不在本研究中授权的事项

- 不在当前 `agent-diva-channels` WebSocket 原型上直接堆完整 Neuro-Link；
- 不把现有 Mate 一次性删除或搬出 GUI；
- 不建立未签名的宿主进程内第三方插件机制；
- 不把 MCP、A2A 或所有 Channel 一次性改写成 PEN；
- 不开始硬件采购或专用设备研发；
- 不修改 BML/Laputa 权威边界；
- 不把研究建议自动升级为已批准实施合同。

正式施工需要按 [路线与门禁](./roadmap-and-gates.md) 拆分独立 Epic、锁、测试和提交。
