# 发布

仅提交本地 `agent-diva-pro`，不 push、不部署。

兼容行为：

- proposal/receipt/GUI 成功 DTO 不变。
- Manager 错误码从合并的 governance conflict 细化为稳定的 version、idempotency、
  already-consumed、expired 和 invalid-transition。
- 新增 `.laputa/candidate-suppression.json`，内容仅为 digest 与生命周期元数据；
  可由 rejected proposals 恢复，不是 Memory authority。
- suppression 只针对完全相同的规范化内容，默认 90 天后失效。

回滚时可以移除 suppression 文件，但 rejected proposal 仍保留治理历史；不得删除
proposal、audit 或 receipt 来伪造回滚。
