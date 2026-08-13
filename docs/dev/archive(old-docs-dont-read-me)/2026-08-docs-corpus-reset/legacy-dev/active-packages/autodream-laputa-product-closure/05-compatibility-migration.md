# 兼容性与迁移

## 1. 数据策略

- typed Laputa 继续是唯一生产 Memory authority。
- 现有 proposals、changelog、audit、AutoDream runs 原地可读。
- 新字段使用 serde 默认或版本化 envelope；未知关键枚举 fail closed。
- 不转换 Memory 内容来完成 schema 迁移；workspace identity 使用独立 migration manifest。

## 2. AutoDream 迁移

旧 run 只作为历史记录，不自动重新执行。状态为 `pending/running` 且没有新版本 stage journal 的 run 标记为 `legacy_incomplete`，GUI 提供“以新 run 重试”，不在原 ID 上猜测恢复点。

旧固定 `journal_note` proposal 保持可审阅，但不视为新 quality gate 已通过；其来源徽标显示 `legacy_candidate`。

## 3. 配置

新安装默认启用 typed Memory，但 AutoDream 自动触发初期默认关闭；手动触发可用。完成成本、安全和恢复基线后，才允许用户显式开启阈值/周期触发。

## 4. 回滚

每个 schema/data 迁移都先备份、生成 manifest、执行完整性校验。回滚只恢复本切片新增的 journal/config/schema，不回退到 legacy Memory，也不恢复 Mentle。
