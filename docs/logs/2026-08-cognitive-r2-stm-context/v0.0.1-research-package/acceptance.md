# Acceptance — COGNITIVE-R2 v0.0.1

- 日期：2026-08-13
- 视角：用户 / 产品阅读研究包，而不是验收实现

## 阅读后应能确认

1. 产品 STM 还没实现；不能把 Memory 页的 `working_memory` 筛选当成 STM。
2. 三条现成链路寿命不同：对话压缩检查点、session scratch、长期 Memory 索引。
3. Research Hold（存储、scope、并发、触发、装配位置、晋升）都有选项，**没有**被本包拍板。
4. 新 session、多 channel、cron、Reset/Delete、崩溃、多进程的缺口已列表。
5. 下一步仍是 R3，然后 R4；不能开始 D2 或改生产代码。

## 用户 Research Gate 动作

- 阅读三份完成物并标出不能接受的选项淘汰（若有）
- 明确是否授权继续 R3；R2 本身不进入架构设计

## 本包不能当作已验收的事项

- STM 物理权威
- Layer 1 装配位置
- `run_startup_gc` 接线
- GUI STM 工作区
- 跨会话真机 smoke
