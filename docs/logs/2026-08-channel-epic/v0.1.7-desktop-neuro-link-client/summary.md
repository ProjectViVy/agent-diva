# Desktop Neuro-Link client and Presentation migration

本迭代完成 CHANNEL-EPIC C4：桌面 GUI 现在通过 Neuro-Link v1 消费实时状态，Mate
和字幕/TTS 由语义 Presentation 事件驱动。

- 补齐 `TurnStartParams.context` 的 owner intent、approval policy、plan revision 和
  execution correlation；AgentLoop 将其映射到既有 turn metadata，外部 Channel payload
  仍保持可选 context。
- Manager projection pump 在同一 request/session correlation 下先落盘并广播语义
  `presentation/event`，再转发基础 conversation envelope；覆盖 thinking、speaking、tool、
  waiting-for-approval、subtitle 和终态清理。
- 新增桌面 typed client/composable：Tauri loopback endpoint、hello/session/open、cursor
  replay、ACK、state/resume gap recovery、idempotent turn start/cancel/close，以及带上限的
  reconnect backoff。cursor 按 workspace/session 隔离，frontend instance ID 稳定且无敏感信息。
- App 的实时 Agent/Reasoning/Tool/Plan/Provider/Compaction/Admission 监听迁移到 Neuro-Link；
  send、regenerate、plan continuation 和 stop 均走 typed request。回放只恢复状态，不重复追加
  chat 消息或触发 TTS；已有流式占位在回放终态到达时会被正确收敛。
- DesktopMateOverlay、DivaMateView 和 subtitle overlay 使用 semantic presentation，删除
  App/DivaMate 对旧 avatar chat-id/`speak` bridge 的调用；TTS 只消费 live 的
  `subtitle.updated`，clear 会停止当前播放。

本迭代没有新增 host/port/auth 配置，也没有删除旧兼容面；C6 仍负责最终 clean break、旧
SSE/DTO/bus 清理和一次性合入 `dev`。真实桌面/Tauri 断线恢复冒烟因当前环境缺少可用浏览器/桌面
自动化执行器，保留到发布工作站验收。
