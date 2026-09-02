# Acceptance

## 自动化验收

- [x] GUI 通过 typed Neuro-Link client 完成 `protocol/hello`、`session/open`、replay/ACK、
      `state/resume` gap recovery 与 bounded reconnect。
- [x] send、regenerate、plan continuation 通过 `turn/start` 携带 owner context；stop 通过
      `turn/cancel`；请求带稳定 frontend idempotency key。
- [x] Manager semantic Presentation 覆盖 thinking/speaking/tool/waiting/subtitle/终态清理，
      并先持久化再广播。
- [x] Mate 与字幕/TTS 只依赖 semantic presentation；replay 不触发 TTS，clear 停止播放。
- [x] `just fmt-check && just check && just test`、GUI tests/build 和 Tauri Rust check 通过。

## 发布工作站人工验收

- [ ] 启动真实 Tauri GUI，确认现有 session 自动 `session/open`，普通消息和重新生成结果正常。
- [ ] 产生排队 turn，执行取消；执行计划批准/继续，确认 owner intent、approval policy 和
      plan/execution revision 在请求链路中保持一致。
- [ ] 在 WS 实时流中断开网络，再恢复连接；确认客户端按 workspace/session cursor 回放，
      不重复 chat 消息、不重复 TTS，并能收敛已有流式占位。
- [ ] 观察 Mate subtitle、TTS speaking 状态、tool lifecycle 和 expression hint；确认
      `subtitle.updated`/`subtitle.cleared` 以及 waiting-for-approval 语义事件可见。

当前工作站未提供 `agent-browser`、可用 Playwright 浏览器或 Edge/Chrome/Firefox 二进制，故以上
人工项尚未宣称通过。完成后应在本文件勾选并补充日期、设备和观察结果，再进入 C6 原子合入。
