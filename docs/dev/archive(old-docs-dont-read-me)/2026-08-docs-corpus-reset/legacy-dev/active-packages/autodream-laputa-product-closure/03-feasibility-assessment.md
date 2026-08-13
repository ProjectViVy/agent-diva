# 可行性评估

## 1. 结论

技术上可行，且大部分高风险底座已经存在。难点不是 SQLite 或 GUI，而是把 LLM 生成的不确定性限制在“候选”层，并用确定性状态机完成恢复、治理与唯一写入。

## 2. 现有资产与缺口

| 能力 | 当前状态 | 复用程度 | 主要缺口 |
|---|---|---:|---|
| typed Memory authority | 已实现 | 高 | workspace identity 与发布演练 |
| proposal repository | 已实现 | 高 | AutoDream 候选质量和 suppression |
| receipt/apply/rollback | 已实现 | 高 | 统一 coordinator 与完整真实 E2E |
| AutoDream run store | 部分实现 | 中 | 队列、阶段恢复、生产 worker |
| 输入收集 | 部分实现 | 中 | outcome 验证、去重、retention |
| 反思生成 | 占位 | 低 | provider adapter、schema、quality gate |
| Recall | 已实现 | 高 | 效果反馈闭环 |
| Evolution GUI | 表面已实现 | 中 | run 到 Memory 的一体化可用性 |

## 3. 关键风险

| 风险 | 等级 | 控制 |
|---|---:|---|
| 模型把未验证推断写成事实 | P0 | evidence gate；候选不可直接成为 authority |
| AutoDream 读取敏感工具输出 | P0 | 采集前脱敏、字段 allowlist、敏感度分类 |
| 重启造成重复 proposal/apply | P0 | stage journal + deterministic keys + replay tests |
| 拒绝候选反复出现 | P1 | digest suppression + bounded expiry |
| 反思成本失控 | P1 | batch/turn/day budget、输入 token 上限、熔断 |
| GUI 显示成功但后台未执行 | P0 | run 状态来自 Manager authority；真实 E2E |
| GenericAgent 直接写经验被误移植 | P0 | 强制 ProposalPublisher→Coordinator→typed writer |

## 4. 方案取舍

推荐“治理式经验蒸馏”。备选的“任务结束立即自动写 Memory”更接近 GenericAgent，但与 Agent Diva 的 authority、审计和用户控制原则冲突，不采用。另一个备选“只做离线月报”无法改善后续 Recall，也不满足产品闭环。

## 5. 资源与依赖

不需要新增数据库或外部服务。优先复用现有 provider、Tokio、SQLite、event bus 和 Tauri。若引入 JSON Schema 校验库，必须确认 Rust 1.80 兼容并在 workspace 统一管理；否则使用 serde 稳定 DTO + 手写业务校验。
