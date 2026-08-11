# C5a Final Wire Prefix Summary

缓存结构观察已移动到 Provider 完成消息转换、工具转换和 cache-control 注入后的最终
请求边界。最终快照只保留 provider namespace、raw model、策略、稳定 system 指纹、
CORE 工具前缀指纹、活动工具名和估算规模，不保存提示正文。

Agent 不再对发送前的稳定前缀和完整工具列表做混合哈希，也不再通过 warmup、连续 miss
或 token 差值推测缓存命中。DEFERRED 工具变化作为声明式活动工具集变化处理，不污染
CORE 前缀指纹。

实现提交：`73dff1bc`；边界与删除证明：`a8747e86`。
