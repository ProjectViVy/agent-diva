# Summary

分批恢复 `stash@{0}` 中尚未存在于当前分支的功能差异，避免整体 pop 覆盖当前分支演进。

- 第一批：Todo、supervised runtime、manager runtime。
- 第二批：subagent handler、CLI、E2E。
- 第三批：agent mask、GUI 组件、story 文档。
- 排除工作树中明显的乱码命令残片和现有 `Cargo.lock` 用户修改。
