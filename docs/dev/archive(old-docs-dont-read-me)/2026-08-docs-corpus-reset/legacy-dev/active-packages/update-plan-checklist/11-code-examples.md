# 重要代码示例

## 规范工具输入

```json
{
  "explanation": "同步当前任务进度",
  "plan": [
    { "step": "定位原因", "status": "completed" },
    { "step": "实现修复", "status": "in_progress" },
    { "step": "执行验证", "status": "pending" }
  ]
}
```

## GUI 完成对账

```ts
const index = completeLatestStreamingAgent(messages, finalContent)
if (index === -1 && finalContent) {
  // 受控回退：创建已完成 assistant 消息。
}
```

实际实现位于 [streamingMessages.ts](agent-diva-gui/src/utils/streamingMessages.ts:19)。输入内容仅作为文本或 JSON 卡片数据处理，不执行脚本、不拼接命令，也不包含密钥。
