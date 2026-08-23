# 伴生节点、生活场景与 Experience 模型

## 1. 产品愿景

目标场景不是“随身摄像头连接聊天机器人”，而是用户携带 DIVA 共同生活：

- 一起吃饭，DIVA 只在被邀请时观察和回应；
- 一起学习，理解教材、学习目标和进度；
- 一起旅行，看见景色、参与对话并帮助记录用户选择保留的时刻；
- 在弱网或离线时仍保有基本陪伴、隐私和连续性。

这个形态暂称 `Companion Node（伴生节点）`。

## 2. 设备结构

~~~text
伴生设备 / 手机 / 眼镜
├─ Mirror
│  ├─ 声音、形象、屏幕、灯光
│  └─ DIVA 的存在感与表达
├─ Sensor PEN
│  ├─ Camera
│  ├─ Microphone
│  ├─ GPS
│  ├─ IMU
│  └─ 环境传感器
├─ Local Guardian
│  ├─ 录制指示
│  ├─ 隐私过滤
│  ├─ 物理关闭
│  ├─ 本地加密
│  └─ 权限与功耗控制
├─ Offline Cache
└─ Neuro-Link Client
          │
          ▼
       DIVA Core
~~~

伴生节点是多种模块的组合，不应被建模为一个万能 Channel。

## 3. 原生能力与外部能力的选择

推荐混合路线：

> 隐私、安全、实时性和最低陪伴能力原生运行；视觉、地图、翻译、搜索等重能力通过
> 可替换 PEN/Provider 接入；设备通过 Neuro-Link 成为完整前端。

### 必须设备原生

- 唤醒词和 VAD；
- 摄像头/麦克风启停与可见指示；
- 物理静音、遮挡或 Kill Switch；
- 本地权限判断和设备身份；
- 原始媒体短期缓冲与自动清除；
- 本地加密；
- 弱网重连与有界离线队列；
- 最小 Mirror 表达；
- 功耗、温度和后台限制；
- 网络失效时仍可查询设备是否在记录。

### 适合外部 PEN/Provider

- 通用视觉、OCR 和场景理解；
- 地图、导航、天气和景点信息；
- 实时翻译；
- 联网搜索；
- 高质量 ASR/TTS；
- 旅行影集和照片处理；
- 医疗、植物、星空等专业识别；
- 相机、眼镜和其他厂商 SDK。

## 4. “一起看晚霞”的调用链

~~~text
用户说：“DIVA，你看这个晚霞。”
  → 本地唤醒和 VAD
  → Neuro-Link UserIntent
  → Camera Sensor PEN 请求一次受控取景
  → Local Guardian 做权限检查和旁人隐私处理
  → Vision PEN 理解景色
  → DIVA 形成回应
  → Neuro-Link Presentation Event
  → Mirror 用声音、字幕、动作或灯光回应
~~~

DIVA 不需要知道视觉来自手机 NPU、本地模型还是云服务；用户仍能知道这次是否采集、
是否上传、保存多久以及如何删除。

## 5. Observation、Moment 与 Memory

生活陪伴必须建立三层边界：

| 层 | 含义 | 默认保留 |
| --- | --- | --- |
| Observation | 当前交互所需的短暂感知 | 不进入长期存储 |
| Moment | 用户主动保存或 DIVA 提议保留的完整时刻 | 暂存并等待确认 |
| Memory | 经明确策略/Laputa 治理进入 BML 的长期记忆 | 按 Memory 合同 |

一起吃饭时，识别菜品可以只是 Observation。只有用户说“记住今天第一次吃这个”，或
DIVA 提议并被接受，才形成 Moment。同行者的人脸、完整录音和精确地点不得因“形成
Moment”被默认打包。

## 6. Experience 模型

建议在 Message、Session、Task、Memory 之外研究一个更丰富、比 Memory 更轻的
`ExperienceEvent`：

~~~text
ExperienceEvent
├─ occurred_at
├─ coarse_location
├─ activity
├─ user_shared_observation_refs
├─ diva_response_ref
├─ atmosphere_hint
├─ retention_class
├─ consent_receipt
└─ proposed_moment
~~~

Experience Store 是短期权威，不等于 BML，也不能由 BML 反向承担原始媒体仓库职责。
它可以在一次学习、用餐或旅行结束后生成可审查的日记/相册提案。

## 7. 感知数据管线

~~~text
Sensor
  → Local Guardian
  → 受限片段或特征
  → Policy / Consent
  → Agent 当前上下文
  → 临时 Observation
  → 可选 Moment Proposal
  → 用户确认 / Laputa 治理
  → BML Memory（如适用）
~~~

原始连续媒体不直接进入 Agent context，不直接进入日志，更不能直接写 BML。

## 8. 三种部署形态

| 形态 | Core 位置 | 适用 |
| --- | --- | --- |
| Sovereign | 手机/设备本地 | 最高隐私、离线生活 |
| Tethered | 家庭电脑或个人服务器 | 设备轻量、能力完整 |
| Hybrid | 本地小节点 + 主 Core | 最现实的长期形态 |

Hybrid 中，设备维持唤醒、隐私、短时状态和最低表达；家庭节点持有主要 Laputa/BML 和
任务系统；云端只提供授权的模型能力。重连后同步用户选择的事件/Moment，而不是全部
原始音视频。

## 9. 第一阶段建议

不先造硬件，以手机作为第一台 Companion Node：

1. 手机前端通过 Neuro-Link 连接自己的 DIVA；
2. 支持按住说话、拍一张和分享粗粒度位置；
3. 感知默认只用于当前 turn；
4. 用户可以说“保存这个时刻”；
5. DIVA 生成可审查 Moment 卡片；
6. 确认后形成旅行日记或受治理记忆；
7. 弱网时本地加密缓存，恢复后继续同一 Session。

跑通这一纵向链路后，再研究胸针、眼镜、桌面机器人或专用随身硬件。

## 10. 伦理与体验原则

- DIVA 的“看见”必须由用户可理解和可控制；
- 默认不持续录制，不以隐蔽采集制造陪伴感；
- 旁观者隐私与设备主人隐私同样需要考虑；
- 所有感知保留级别都可查看、撤销和清理；
- DIVA 可以参与和表达，但不能伪造自己拥有未经记录的真实体验；
- 生活陪伴的核心是共同选择的时刻，不是最大化数据收集。
