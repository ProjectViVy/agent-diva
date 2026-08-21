# Workspace 与 AGENTS.md：Codex 对照研究总结

## 交付范围

本迭代只做本机 `.workspace/codex` 与 Diva 当前代码的事实核对和 Diva 化方案，不修改 Rust、
Tauri、配置或构建文件。

## 主要结论

1. Codex 以当前 CWD 为默认工作根，`--cd`/thread `cwd` 贯穿相对路径、Sandbox、session
   和 AGENTS 发现；Diva 当前 CLI 已有 `--workspace`，但默认是 `~/.agent-diva/workspace`。
2. Diva 的 workspace 传播链已经存在；下一步是统一 canonical root、避免全局 chdir、限制
   Shell `working_dir` 越界，并分离机器级 `~/.agent-diva` 与项目工作区。
3. Diva 已经实现根 `AGENTS.md` 注入与缺失无处理；本次只建议补来源/预算/刷新和安全边界，
   不复制 Codex 的完整层级项目文档系统。
