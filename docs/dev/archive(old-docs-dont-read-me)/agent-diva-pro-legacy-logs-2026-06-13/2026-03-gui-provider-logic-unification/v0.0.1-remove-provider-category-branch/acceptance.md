# Acceptance

1. 打开 GUI，进入 `设置 -> 供应商`。
2. 确认供应商列表不再显示“本地 / 网关 / 云端”分类标签。
3. 选择任一此前被标记为“网关”的供应商，填写 API Key / API Base 后刷新模型。
4. 确认模型列表可正常展示，且选择模型后保存的 provider/model/api_base 组合不再因为分类逻辑被改写到特殊分支。
5. 对比一个常规 OpenAI-compatible provider 与一个此前“网关” provider，确认它们在 GUI 配置流程中的行为一致。
