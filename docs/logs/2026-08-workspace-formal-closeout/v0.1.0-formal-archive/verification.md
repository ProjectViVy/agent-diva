# Verification

## Documentation checks

- `git diff --check` — 通过。
- 活跃 `TODOLIST.md` 不再包含已完成的 WS-00～WS-06 明细或重复 Workspace 条目。
- 活跃 `TODOLIST.md` 保留 `WS-CLI-LEGACY-DEFAULT-MIGRATION`，避免把独立兼容性缺口误报为已完成。
- 归档正文、归档索引和本迭代四件套文件均已落盘。

## Repository state checks

- `git worktree list` — 仅保留根目录 `dev` worktree。
- `git branch --list "*workspace*" "*m3*"` — 无待退休的对应本地分支。
- `LOCK.md` — 完成后释放；与本次无关的既有 dirty 文件保持不变。

本迭代为文档/归档变更，未重复运行 Rust 或 GUI 产品门禁；相关产品门禁结果沿用归档的
`v0.1.7-workspace-closeout` 与后续 Workspace 验收记录。
