# 部署与发布方案

发布前执行 `just fmt-check`、相关 Rust 测试、`just check` 和 GUI 构建。灰度时默认维持 `on-failure`；出现审批协调器故障时拒绝执行并保留审计记录。
