# Release

本迭代仅在本地 `agent-diva-pro` 分支提交，不 push、不部署。C5b 是未发布内部压缩
metadata 的 clean break：不读取旧 compaction metadata，不双写、不保留兼容反序列化，
完整 transcript 仍是审计来源。

后续 C5c–C5e 独立实施，不与本次发布单元混合。
