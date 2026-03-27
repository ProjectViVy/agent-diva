# Acceptance

## 用户视角验收

- 询问“之前/最近/结论/偏好”类问题时，系统默认依赖 compact recall，而不是把整个 `MEMORY.md` 注入到主 prompt。
- 当 `brain.db` 缺失或损坏时，重启后能从 diary 文件、`MEMORY.md` chunks、snapshot 恢复可查询记录。
- `memory_recall` 继续可用，并在 semantic 不可用时平稳退化为关键词召回。
- 新增 `memory_search` 可返回 snippet、source、时间与定位信息。
- 新增 `memory_get` 可按 `id` 或 `source_path` 返回完整记录或来源片段。
