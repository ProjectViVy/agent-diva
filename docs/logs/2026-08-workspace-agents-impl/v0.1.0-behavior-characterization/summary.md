# v0.1.0 行为表征测试：总结

> 状态：骨架（Wave B 完成后补全）

## 交付范围

Phase 0 行为表征：只加测试、不改行为，为后续 WorkspaceContext 改造建立回归基线。

## 计划内容

- `agent-diva-cli/tests/`：`effective_workspace()` 覆盖 default / `--workspace` 显式 /
  相对路径 / 带 `~` 路径四种情况（当前行为快照）。
- `agent-diva-agent`：AGENTS.md 注入矩阵（缺失不追加、存在注入一次、空文件、超 4000
  字符截断、变更后未 invalidate 用旧缓存、invalidate 后刷新）。
- `agent-diva-tools`：Shell `working_dir` 越界负向测试（新合同生效前先标注待转正）。
