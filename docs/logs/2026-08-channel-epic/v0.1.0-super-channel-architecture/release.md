# CHANNEL-EPIC C0 发布说明

## 发布方式

本批是文档型 C0 交付：在 `dev` 上提交一笔聚焦 Conventional Commit，不构建或发布二进制，
不迁移配置和数据，也不推送远端。

建议提交信息：

```text
docs: start channel epic architecture
```

## 后续开发方式

- C1～C6 使用隔离 `feat/channel-epic` worktree。
- 中间提交不得把双轨产品状态合入 `dev`。
- C6 完成全部 TCK、GUI、MSRV、clean-break 和 release gate 后原子合并。

## 回滚

本批没有运行时影响。若架构记录需撤回，回退本次文档提交即可；不得以回退文档为理由
恢复已经被关闭的旧研究决策或绕过后续正式评审。
