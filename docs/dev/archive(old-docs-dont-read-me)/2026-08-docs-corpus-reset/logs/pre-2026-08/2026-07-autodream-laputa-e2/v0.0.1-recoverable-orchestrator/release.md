# 发布

本切片仅提交到本地 `agent-diva-pro` 分支，不 push、不部署。

兼容策略：

- 新运行始终带 orchestration 记录。
- 已完成的旧运行仍可读取。
- 缺少 orchestration 的旧版未完成运行无法证明安全续跑，因此以
  `legacy_incomplete` 明确失败，不产生提案或 Memory 写入。
- Manager API 的运行 DTO 只增加字段；触发请求结构不变。POST 响应现在表示
  queued 接收结果，终态通过既有状态查询获取。

回滚本提交会恢复同步触发行为，但不应删除已经生成的运行记录或 Laputa 提案。
