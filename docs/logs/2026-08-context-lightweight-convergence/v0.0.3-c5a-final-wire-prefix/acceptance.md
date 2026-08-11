# Acceptance

1. OpenAI-compatible 与 Anthropic 请求在最终塑形后生成缓存结构快照。
2. 快照保留 raw model，不对 native provider 添加 gateway 前缀。
3. 修改 DEFERRED 后缀不会改变 CORE 工具前缀指纹。
4. Provider 内部 retry 不重复推进结构观察状态。
5. Provider 未返回 cache usage 时结果保持 unknown，不推测 hit/miss。
6. 旧 warmup、连续 miss 和完整工具混合哈希状态无法通过删除证明。
