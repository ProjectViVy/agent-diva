# Acceptance — GUI Provider 错误/重试可见性修复

- 版本：`v0.1.0-provider-error-retry-visibility`
- 日期：2026-08-06

## 验收场景（用户视角）

### A. 原 bug 场景：Provider 连接失败重试后最终失败（`GUI-PROVIDER-ERROR-SILENT`）

1. 配置一个不可达的 Provider api_base（或断网），在 GUI 发送任意消息。
2. **重试期间**：流式占位消息上出现「未响应，重试 (1/3)」→「(2/3)」→「(3/3)」
   琥珀色徽标，逐次更新（对应约 1s/2s/4s 退避）。
3. **等待期间**（若 60s 无任何事件）：出现「仍在等待 Provider 响应...」提示。
4. **最终**：出现明确错误气泡——重试耗尽的 Provider 错误，或
   「长时间未收到响应，连接已断开。请检查 Provider 状态或网络后重试。」
   ——不再是无提示静默失败。
5. 错误后输入框恢复可用（`isTyping` 释放），可继续发送消息。

### B. 增强场景：重试可见性（`GUI-PROVIDER-RETRY-VISIBILITY`）

1. Provider 瞬时抖动（如 5xx）后恢复：重试徽标出现 1-2 次后消失，
   最终回复正常送达，无错误气泡。
2. 徽标随回复完成自动消失，不残留。

### C. 无回归

1. 正常对话：无重试徽标、无错误气泡、流式输出/工具调用正常。
2. CLI 不受影响：`agent-diva chat` 正常；headless 无挂起。

## 记录

- 人工 smoke 执行结果记录到 `verification.md`；
- 两条 sev-P2 TODO 移至 done；
- 新增 TODO：plan 流式 Tauri command 断流兜底统一（本次未覆盖 L2854/L3140）。
