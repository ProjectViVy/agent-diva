# Acceptance

1. 执行小型工具后，provider 看到完整 inline result。
2. 执行大工具后，provider 看到版本化 artifact ref/preview，完整结果可由绑定 session 的
   `read_tool_result` 读取。
3. artifact 容量或 IO 失败时，模型看到明确错误码，不看到伪造的部分成功正文。
4. subagent 与 microcompact 使用相同表示，重复 microcompact 不重复写入或改变结果语义。
