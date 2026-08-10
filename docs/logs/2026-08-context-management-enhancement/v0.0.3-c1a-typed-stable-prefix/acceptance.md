# C1a Acceptance

## 自动验收

1. 运行 T1–T3，确认时间、WM、Recall 变化仅改变动态 envelope。
2. 运行 provider 测试，确认现有 adapter 使用安全 user envelope，显式 mid-system 可测，
   Native transport fail closed。
3. 运行 compaction E2E，确认 overflow retry 保留同一动态快照和 current user 位置。
4. 运行 `just fmt-check`、`just check`、`just test`，三门必须全绿。

## 产品观察点

- 首条 system 不再含 `## Current Time` 或 `## Current Session`。
- provider 调用中存在带 `<agent_diva_context section="...">` 边界的 post-prefix user
  message；最后一条初始 turn message仍是原始 current user（含多模态 parts 时保持结构）。
- Plan/Ask/Scheduled 行为与工具权限门保持原语义。

## 完成定义

C1a 只在上述条件全部满足时完成；cache 命中率提升的实测与告警不属于本切片，留给
C1d 观测闭环。
