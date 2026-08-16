# S5 发布说明

不适用（N/A）。S5 是纯拆除切片：无新增运行面、无配置迁移、无部署产物；
clean-break 政策明确不做运行时兼容/回退。

后续：

- 证明门 `just cognitive-clean-break-check` 与桌面 UI smoke 由 **S6** 交付并作为
  发布判据。
- 本分支（`agent-diva-pro`）按项目规则未 push；是否 push 由用户决定。
- 回滚路径：整体回退用本地保护分支 `protect/cognitive-pre-clean-break-20260815`
  （`2aab18cc`）或按提交逐笔回退 `710e7684`..`1ea54d08`。
