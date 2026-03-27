# v0.0.6 Verification

## Validation Commands

由于当前 macOS 环境下 `justfile` 仍依赖 `powershell.exe`，本轮继续使用等价 `cargo` 命令进行验证。

- `cargo fmt --all`
- `cargo test -p agent-diva-memory -- --nocapture`
- `cargo test -p agent-diva-agent loop_turn:: -- --nocapture`
- `cargo test -p agent-diva-agent context:: -- --nocapture`
- `cargo test -p agent-diva-tools memory:: -- --nocapture`
- `cargo fmt --all -- --check`
- `cargo clippy --all -- -D warnings`
- `cargo test --all`

## Results

- `cargo fmt --all`：通过
- `cargo test -p agent-diva-memory -- --nocapture`：通过
- `cargo test -p agent-diva-agent loop_turn:: -- --nocapture`：通过
- `cargo test -p agent-diva-agent context:: -- --nocapture`：通过
- `cargo test -p agent-diva-tools memory:: -- --nocapture`：通过
- `cargo fmt --all -- --check`：通过
- `cargo clippy --all -- -D warnings`：通过
- `cargo test --all`：通过

## SQLite-Specific Coverage

- `SqliteMemoryStore` 会创建 `memory/brain.db`
- schema 初始化可重复执行
- `MemoryStore` 的基础 CRUD 可正常工作
- `tags` / `source_refs` 的 JSON 序列化与反序列化通过
- 缺失记录删除返回 `false`
- SQLite store 接入后，现有 recall policy 与 tool contract 未回归

## Environment Notes

- 新增 `rusqlite` 依赖后，首次构建需要访问 crates.io 索引。
- 本轮曾因磁盘空间不足导致编译失败；已清理旧 worktree 的 `target/` 可再生产物后恢复验证。
- 未清理任何 repo 跟踪文件或与本轮无关的源码改动。
