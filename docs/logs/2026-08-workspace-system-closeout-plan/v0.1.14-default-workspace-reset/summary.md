# 默认工作区目录配置与重置

本阶段将两个此前混用的概念拆成独立 authority：

- “设置 → 工作区”只管理全局 `agents.defaults.workspace`，保存或重置不会改变当前 runtime
  及其 session authority。
- 聊天入口选择目录会切换到目标 workspace 的 session authority，不会改写既有 session 的
  workspace 归属，也不再写入默认工作区配置。
- 未显式选择工作区时使用全局默认目录；显式选择后 runtime source 标记为 `explicit-cli`，
  入口显示所选目录名。
- Diva 内置默认目录固定为 profile 配置目录下的 `workspace`；legacy GUI 默认值启动时也投影
  到该稳定目录，不再继承 `tauri dev` 的进程 CWD。
- 默认目录设置提供独立读取、保存和重置 Tauri API；重置会创建并保存内置默认目录。
- 工作区切换失败时同时恢复旧 runtime 的路径和 source，避免默认工作区在回滚后被误标为
  session 显式目录。

根据 `oil-frontend` 的数据归属规则，全局默认配置和当前 runtime/session 状态分别由各自的数据源与
操作维护，不通过文案伪装成同一个字段。
