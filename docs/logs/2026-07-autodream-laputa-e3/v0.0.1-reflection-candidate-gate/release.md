# 发布

本切片仅在本地分支提交，不 push、不部署。

兼容与运行行为：

- AutoDream trigger、run、proposal 的已有请求结构保持不变。
- run/artifact 增加候选元数据与稳定 provider/candidate failure code。
- 未配置可用 provider 时不再生成规则式占位提案；运行明确以
  `provider_unavailable` 或 `provider_failed` 结束。
- provider 正常但没有可蒸馏事实时，运行完成且 proposal 数量为零。
- report curation 与 Reflection 使用各自的用途契约；Reflection 不启用工具。

回滚本提交会恢复占位候选行为，因此只能连同 E3 运行记录语义一起回滚。已创建的
Laputa proposal 不应被文件删除替代正式治理操作。
