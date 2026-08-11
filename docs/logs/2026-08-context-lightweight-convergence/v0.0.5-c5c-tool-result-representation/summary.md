# C5c Canonical Tool Result Representation

C5c 将工具结果收敛为单一 `CanonicalToolResult`：小结果以内联正文表示，较大结果写入
session/workspace 绑定的 artifact，并向 provider 发送版本化引用与受限 preview。artifact
容量、IO 或渲染失败会返回明确的 materialization error，不再把部分正文静默截断为旧式
safety fallback。

主 agent、subagent、microcompact 和 registry execution 均使用同一生产入口；完整脱敏结果
仍由 registry 先生成，原始 transcript 继续保留，artifact 生命周期继续由既有安全存储管理。

实现提交：`6f1331f0`。
