# Windows 工作区路径展示修复

Windows canonicalize 返回的 verbatim path 会带 `\\?\` 前缀。该前缀对文件系统调用有意义，
但不应作为工作区身份展示给用户。

- 新增共享展示格式化函数，将 `\\?\C:\...` 显示为 `C:\...`。
- 将 `\\?\UNC\server\share` 显示为熟悉的 `\\server\share`。
- WorkspaceChip、工作区设置当前路径、候选路径和 AGENTS.md 来源路径统一使用展示格式。
- inspect 和原子切换继续使用后端原始路径，不改变路径 authority。

`oil-frontend` 规则使本次改动落在共享展示层，而不是修改工作区数据或在单个模板里追加补丁。
