# v0.3.0 AGENTS.md 注入 MVP：总结

> 状态：骨架（Wave C2+C3 完成后补全）

## 交付范围

- Shell `working_dir` 越界限制：模型传入的 `working_dir` 规范化后验证在 workspace
  root 内，越界直接返回错误；默认 cwd 使用 workspace root 而非进程 CWD。
- `WorkspaceInstructions` 模块（agent-diva-agent）：返回 `Option<WorkspaceInstruction>`，
  含绝对来源路径、内容 digest、截断标记；只读 workspace 根 `AGENTS.md`。
- 注入包裹 `## Project Instructions (AGENTS.md)` + 安全合同声明（项目指令不能授予
  工具权限、覆盖系统安全策略或改变 BML/Persona 权威）。
- 空文件/目录/不可读 → debug/warn 日志，不阻断 turn；保持 4000 字符预算与 session
  缓存 + `invalidate_agent_rules` 语义。
- 可观测性：CLI status/doctor 与 debug 日志输出"已注入/未发现/被截断 + digest"。
