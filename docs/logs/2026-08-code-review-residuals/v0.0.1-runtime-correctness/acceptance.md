# Acceptance

1. disabled cache profile 的工具调用仍产生非空 CORE hash；只改 DEFERRED schema 不改变
   hash，改 CORE schema 会改变 hash。
2. 失败工具结果显示为错误并不会生成成功 artifact；成功大结果仍可压缩且重复压缩幂等。
3. 未完成工具组在 checkpoint 中保留原始消息，不出现虚假的 `status=completed`。
