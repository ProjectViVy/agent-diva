# v0.3.0 Shell working_dir 越界限制总结（Wave C2）

## 交付

- `agent-diva-tools/src/shell.rs`：
  - 新增 `enforce_workspace_boundary(target, scope)`：canonicalize 后验证
    `starts_with`，越界返回 `Err` 并附清晰消息
  - 新增 `canonicalize_best_effort`：对尚未创建的路径沿祖先链 canonicalize
  - `ExecTool::execute` 重构 working_dir 解析链：
    - 模型传入 > 工具配置默认 > workspace scope > 进程 CWD
    - 相对 working_dir 以 workspace 为基准解析
    - 当存在 workspace scope 时强制边界检查
  - 激活之前 `#[ignore]` 的负向测试；新增 `working_dir_relative_escape_is_rejected`
    与 `working_dir_inside_workspace_subdir_runs`
- 无 workspace scope 的工具保持旧 CWD 回退行为

## 影响

- 模型无法再将 shell 执行切到 workspace 外部目录（含 `..` 穿越）。
- 越界被拒绝时返回带 "workspace" 关键词的消息，供上层观察和审计。
- 不影响无 scope 工具与单测。

## 验证

- `cargo test -p agent-diva-tools --lib shell`：18/18 通过（含 3 个新测试）。
- `cargo fmt --check` / `cargo clippy --lib -D warnings`：干净。
