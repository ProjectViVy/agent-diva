# GenericAgent 优雅接入与用户可控学习调研总结

## 版本信息
- 版本号: v0.0.1-research-synthesis
- 调研日期: 2026-05-27
- 范围: `agent-diva-selfinprove`、`.workspace/memtle`、`.workspace/agent-diva-vrm-memory-test`、`laputa-work`
- 约束: 只读调研，不修改业务代码

---

## 目标与结论

本轮目标是验证以下路线是否可行：

1. 优雅吸收 GenericAgent 的核心价值（分层记忆、公理化学习、可验证沉淀）；
2. 不引入 laputa 全栈复杂度；
3. 采用“节律 + 定时提问 + 用户确认”替代自动学习沉淀；
4. 以 `MEMORY.md` 作为 Compass，mentle 作为事实/证据记忆层；
5. 增加索引层实现“房间/抽屉”快速导航。

结论：该路线高可行，且 `agent-diva-vrm-memory-test` 已提供关键骨架（`MemoryProvider` 边界、prefetch/sync_turn/session_end 生命周期、mentle feature-gate、降级回退）。

---

## 核心架构判断

### 1) 当前主分支问题点
- 主分支主要是 `MEMORY.md` 全量注入 + consolidation 写回 `MEMORY.md/HISTORY.md`。
- 缺“用户确认学习”链路，缺“索引优先召回”链路。
- 缺统一的学习候选状态机与审计/撤销机制。

### 2) vrm-memory-test 提供的关键价值
- 已将 memory 提升为 `MemoryProvider` 抽象边界。
- 已实现 intent-aware `prefetch`、`sync_turn`、`on_session_end` 生命周期。
- 已有 mentle runtime 与工具注册、异常回退（disable/fallback）机制。
- 能直接承接“索引 + mentle + 用户可控学习”方案。

### 3) laputa 可借鉴边界
- 借鉴“节律/投影/摘要”思想，不建议当前阶段接入完整状态机。
- 当前阶段只需“轻 rhythm + 用户确认决策网关 + 索引层”。

---

## 用户可控学习方案（最终收敛）

### 分层职责
- `MEMORY.md`: Compass（身份、关系、高层规则）
- mentle: 深层事实与证据（房间/抽屉/KG）
- Learning Index: 导航层（active_rooms/hot_drawers/open_threads/capsule_pointer）
- Learning Inbox: 候选缓冲层（待询问、待确认）

### 学习链路
1. 采集候选（会话、工具结果、任务总结）
2. 入 Inbox（不自动落库）
3. 节律触发提问（每日/每N轮/周回顾）
4. 用户决策（SOP/Skill/Fact/History/Discard）
5. 按决策写入 mentle 与索引
6. 召回时先查索引，再做 mentle 定向检索，最后回退 Compass

---

## 待落地的数据规格（已定义）

- LearningCandidate: `candidate_id/session_id/turn_id/content/evidence_refs/suggested_type/confidence/verification_state/status`
- LearningDecision: `decision_id/candidate_id/decider/decision/reason/target_room/target_drawer/decided_at`
- 状态机: `inbox -> asked -> accepted|rejected -> archived`

---

## 风险与治理

### 重点风险
1. 索引与 mentle 内容漂移
2. 未验证信息污染（自动写入）
3. 询问频率过高影响体验
4. 双写一致性与回滚复杂性

### 控制策略
- No Execution, No Memory（未验证不沉淀）
- 索引仅存指针，不存大段正文
- 节律询问限频（静默时段、每日上限、冷却时间）
- 决策审计与撤销/重分类全链路记录

---

## 推荐后续阶段

1. 先实现 Learning Inbox + Decision 模型
2. 接入节律提问（用户确认）
3. 接入索引层与 mentle 映射
4. 接入索引优先召回路由
5. 增补治理（审计、撤销、重分类、阈值）

本版本为调研沉淀文档，不含代码变更。
