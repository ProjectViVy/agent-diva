# verification.md

## 验证方式

- 文档归档校验：确认提案文件路径、README 索引条目、交叉链接路径可读。
- 未运行 `just fmt-check` / `just check` / `just test`：本迭代仅 Markdown，无 Rust/GUI 行为变更。

## 结果

| 项 | 结果 |
| --- | --- |
| `docs/research/sandbox-hitl-approval-policy-proposal.md` 存在 | 通过 |
| `docs/research/README.md` 含新条目 | 通过 |
| 既有调研文首交叉链接 | 通过 |
| 代码/CI 门禁 | 跳过（docs-only，有意） |
