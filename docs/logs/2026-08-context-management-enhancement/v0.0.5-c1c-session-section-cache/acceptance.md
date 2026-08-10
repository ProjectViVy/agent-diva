# C1c Acceptance

## 自动验收

1. 同 session 连续装配复用缓存，不重复调用 startup memory；不同 session 独立捕获。
2. 修改 AGENTS/Skills 文件不会自动改变既有快照；显式 reload 只刷新组合 section。
3. T5：mask 切换只改变 mask section，版本加一且 reason 为 `mask_changed`。
4. T6：同运行时 L1 revision 只改变 memory section，版本加一且 reason 为
   `l1_hot_refresh`。
5. reset 重新捕获全部稳定 section；delete/session end 清缓存；compact 不清缓存。
6. 运行受影响 crate 测试、严格库级 Clippy、CLI smoke 与工作区三门。

## 完成定义

C1c 在 T5–T6、生命周期测试、文档与工作区门禁全部通过后完成。Manager 外部 apply
和 Skills 管理入口通知属于已登记后续，不允许改用每轮文件系统轮询。
