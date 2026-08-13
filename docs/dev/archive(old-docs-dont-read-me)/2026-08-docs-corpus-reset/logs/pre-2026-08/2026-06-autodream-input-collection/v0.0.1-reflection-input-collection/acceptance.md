# Acceptance

## User-Facing Checks

1. 手动创建一个 AutoDream run。
2. 为工作区准备最近 session、Laputa section，按需放入 `.agent-diva/compact/capsules/`。
3. 调用 `AutoDreamService::collect_inputs(run_id)`。
4. 确认 `runs/{run_id}/record.json` 包含 `input_summary`，并能看到 source summary、omissions、total bytes。

## Boundary Checks

1. 缺失 session、缺失 capsule、缺失 Laputa section 不应导致整个采集失败，除非所有必需输入都不可用。
2. `.laputa/` authority 文件内容不应被输入采集改写。
3. 不应创建或写入 `MEMORY.md`、`.mentle/` 等 authority/兼容层路径。
