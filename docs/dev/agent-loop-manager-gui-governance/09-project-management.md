# 项目管理与执行顺序

## 1. 治理原则

这是技术债治理计划，不是产品重写。每个 story 必须同时写明：

- 删除/收口了哪项责任；
- 保持了哪些行为契约；
- 新增了哪些测试证据；
- 是否减少 authority source、执行路径、DTO 副本或 host-specific domain rule。

LOC 只作为报警器，不是完成标准。

## 2. Work Breakdown Structure

### G0：基线与门禁

- [ ] G0.1 主旅程 characterization tests
- [ ] G0.2 event/DTO/route snapshots
- [ ] G0.3 GUI capability ledger
- [ ] G0.4 性能与结构基线

### G1：Agent Loop

- [ ] G1.1 提取 `TurnSnapshot`，统一 policy phase
- [ ] G1.2 收口 tool execution seam
- [ ] G1.3 拆 admission/context
- [ ] G1.4 拆 model iteration/tool step
- [ ] G1.5 拆 finalize/persistence/events
- [ ] G1.6 删除旧巨型实现并做全旅程 smoke

### G2：Manager

- [ ] G2.1 拆 chat/session/events handler
- [ ] G2.2 拆 config/provider/tools handler
- [ ] G2.3 拆 skills/MCP/files/cron handler
- [ ] G2.4 DTO 与 `ApiError` 分域
- [ ] G2.5 runtime bootstrap 成为唯一 composition root
- [ ] G2.6 `Manager` façade 委托显式 service

### G3：GUI API 与 Host

- [ ] G3.1 建立 API barrel/domain adapters
- [ ] G3.2 分类 `MANAGER/LOCAL/DEFERRED/REMOVED`
- [ ] G3.3 Tauri LOCAL commands 分模块
- [ ] G3.4 Manager proxy/event bridge 分模块
- [ ] G3.5 禁止 component 直接新增 invoke
- [ ] G3.6 删除已迁 domain 的旧 `desktop.ts` 实现

### G4：GUI 状态

- [ ] G4.1 提取 session/chat stream composables
- [ ] G4.2 提取 plan projection composable
- [ ] G4.3 提取 provider/config/health composables
- [ ] G4.4 `App.vue` 退化为 shell/composition
- [ ] G4.5 `ChatView.vue` 退化为 view + intent

### G5：协议与收尾

- [ ] G5.1 Rust/TS fixture contract tests
- [ ] G5.2 去除高频 DTO 副本
- [ ] G5.3 决定是否引入 codegen ADR
- [ ] G5.4 删除兼容壳与 dead paths
- [ ] G5.5 全量 CI、GUI smoke、性能回归

## 3. 依赖顺序

```text
G0
 ├─> G1 ─> G2 chat/session
 └─> G3 API/Host ─> G4
G2 + G3 ─> G5 contract cleanup
```

允许 G1 与 G3 在文件完全不重叠时并行；G2 chat/session 必须等 G1 的 turn/event seam 稳定。G4 必须等对应 G3 domain adapter 可用。

## 4. 工期区间

| 阶段 | 单人估算 | 最大不确定性 |
| --- | ---: | --- |
| G0 | 1–2 周 | fixture 与现存 flaky tests |
| G1 | 3–5 周 | async borrow、事件顺序、Plan |
| G2 | 2–4 周 | DTO/route 兼容 |
| G3 | 3–5 周 | 134 级别 command 面与 LOCAL 分类 |
| G4 | 3–5 周 | App/Chat stream 状态 |
| G5 | 2–3 周 | 协议漂移和全量回归 |
| 合计 | 14–24 人周 | 并行冲突与未记录行为 |

这比 deep 评估的 C-Core 低，是因为本方案不重建 domain、Gateway、storage 和产品能力，只治理现有实现。

## 5. 单 story 尺寸门禁

- 一次只治理一个 seam/domain；
- 不跨 AgentLoop、Manager、GUI 三层同时大改；
- 生产改动建议不超过约 800 行净变更，超出必须拆 story；
- 每个 story 至少一个 failure-path test；
- 每个 story 有独立日志目录与可回滚提交；
- 结构变更与行为修复分提交。

## 6. 实现授权

当前状态：**Research/Design Complete，Implementation Not Authorized**。
开始 G0 也需要明确实现任务；本文不把研究完成解释为可以自动修改运行时代码。
