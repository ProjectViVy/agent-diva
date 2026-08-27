# v0.3.0 AGENTS.md 注入 MVP：发布说明

> 状态：骨架（Wave C2+C3 完成后补全）

行为变化：

- Shell 工具的 `working_dir` 参数不再允许跳出选定 workspace 根目录（越界即拒绝）。
- 系统提示中 AGENTS.md 段落标题与包裹声明变更（`## Project Instructions (AGENTS.md)`）。
- 重启 gateway/CLI 后生效。
