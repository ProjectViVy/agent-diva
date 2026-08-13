# Verification

## 文档核验

- 对照根 `TODOLIST.md` 的 Active Plan、Product/Architecture、GMH、Reliability、
  Plan Residual 和 Deferred Product 分类。
- 对照治理核心、Laputa 最终架构、Skill/SOP 统一设计、RG-CODE-GOV 项目计划和
  EVO-DIVA 活跃文档入口。
- 确认蓝图未授权本次运行时代码修改，且保留 G2D 作为当前唯一硬门。
- 确认所有现有开放分类都被映射至 B0–B8 至少一个批次。

## 不适用的验证

本切片只改 Markdown，不运行 Rust、GUI 或 Tauri 构建。代码验证不会为文档规划
增加有效证据；Markdown 链接和 git diff 在提交前检查。
