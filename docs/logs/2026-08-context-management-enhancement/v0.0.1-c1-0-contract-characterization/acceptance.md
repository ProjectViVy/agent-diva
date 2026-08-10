# 验收

1. `context_assembly` 暴露最小 section/stability 契约。
2. 固定逻辑顺序保证稳定前缀连续位于所有 volatile section 之前。
3. 测试明确记录当前首 system、Plan、WM/Recall 和工具 definitions 行为。
4. 生产 `ContextBuilder` 与 provider wire shape 未切换到新契约。
5. 受影响 crate 与工作区正式门全部通过。

C1 可在此基础上先实施 C1a 布局迁移，再推进稳定工具排序、section cache 与缓存观测。
