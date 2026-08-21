# Workspace 与 AGENTS.md：用户验收

进入施工前请确认：

1. 未指定 workspace 时，默认采用进程当前目录，还是继续采用 `~/.agent-diva/workspace`；
2. 指定 workspace 后，是否要求 Shell 的显式 `working_dir` 必须位于该根目录内；
3. 选择外部 workspace 时，是否禁止启动自动创建 `PROFILE.md`、`TASK.md`、`masks/`；
4. AGENTS MVP 是否只读取 workspace 根文件，Codex 的父子目录层级扫描留到后续阶段。

当前交付状态：研究包完成，未授权生产实现；没有新的 CLI/GUI 冒烟结果可声明。
