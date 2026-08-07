# LAPUTA-COGNITIVE-SYNC 验收

## 决策符合性（Q1–Q5 + D1/D2 冻结决策逐条核对）

| 决策 | 要求 | 落地状态 |
|------|------|----------|
| Q1 | 认知文件 workspace 级 `.laputa/cognitive/` | ✅ MEMRULES.MD + WORLD.MD 缺省种子，InitializeDir 永不覆盖 |
| Q2 | `LaputaSectionName` 5 废弃变体一步硬删 | ✅ 14→8；落盘历史 unknown-variant 稳定失败/容错跳过 |
| Q3 | 删除人格文件层，完全 Laputa 治理 | ✅ SOUL/IDENTITY/USER/BOOTSTRAP/MEMORY/HISTORY 退役；内容走治理审批迁移入 `.laputa/legacy/`；AGENTS.md/Skills 保留 |
| Q4 | WORLD.MD AutoDream 首切片即可治理写 + confirmed+user 保护 | ✅ `WorldGovernance` upsert 管线；保护=仅可标 stale+追加备注 |
| Q5=b | 报告系统完全重构（边界四句） | ✅ 报告≠记忆（Daily/Weekly/Monthly 映射删除）、永不注入（Rhythm Signals 移除）、agent 可选读自决（仅 read_file）、生成权威=report_system |
| D1 | Mask 保留为覆层，不进 Laputa 治理 | ✅ 装配序 Mask 覆层→Frozen Core 本体；Mask 不解除 commitment 红线 |
| D2 | 报告产物迁 `.laputa/reports/`，只搬不改可回滚 | ✅ open() 一次性迁移，幂等，冲突保留 legacy 副本作回滚源 |

## 负向回归验收（S7 即整体回归验收）

`agent-diva-laputa/tests/context_plane_invariants.rs` 8/8 通过，覆盖 ADR-0004 §6
全部 8 行矩阵（详见 verification.md）。prompt 装配面不再读取任何退役/认知/报告层。

## 遗留与后续

1. **S6-4 完整 daemon-cron 真机**：进程级集成测试已覆盖触发链路；
   GUI 按钮 + cron 实际落盘的真机验证受环境限制（需 gateway + LLM），
   挂入 G2D+ 桌面最终验收补跑。
2. **`.laputa/sections/` 与 `.laputa/proposals/` 残留旧文件**：按设计惰性保留，
   只读路径容错跳过，不做清理。
3. **分支未 push**：`feat/laputa-cognitive-sync` 本地 15 笔提交待用户评审合并。
4. 历史 docs（PRD/旧 logs）中的旧路径描述按约定不追溯修改。
