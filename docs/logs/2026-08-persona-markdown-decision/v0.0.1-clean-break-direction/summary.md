# Summary

记录 Persona 领域的 Markdown clean-break 决策：

- 四个 Frozen Core 人格对象的正文全面改为 Markdown；
- GUI 使用人格导航、源码编辑器、人类预览和文本 Diff；
- 用户手动编辑直接保存并生成历史/审计，不在 Persona 页面二次审批；
- 删除人格 JSON 权威、JSON patch、JSON 编辑 UI、Governance 生命周期和全部旧人格文件
  兼容链路；
- 破坏性实施前必须建立保护性分支。

本迭代只记录决策和活跃待办，没有修改产品代码或用户数据。
