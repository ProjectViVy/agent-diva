# Workspace 默认标签逻辑

## 变更

- `process-cwd` 与 `legacy-default` 来源统一显示“默认工作区”。
- `configured` 与 `explicit-cli` 来源显示所选 canonical root 的目录名称。
- Popover 继续展示真实 canonical root 和来源，不根据路径反推配置状态。

## 影响

聊天底栏不会把进程启动目录误呈现为用户主动选择的工作区。
