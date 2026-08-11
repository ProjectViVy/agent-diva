# C5 轻量上下文收敛与 Clean Break 施工规格

状态：已批准进入计划，尚未实施生产代码

日期：2026-08-11

适用范围：C1–C4 新增的上下文缓存、压缩、工具结果引用和工具发现路径

## 1. 决策摘要

C5 不再只是“长任务验收”，而是一次有界的上下文架构收敛。目标模型固定为：

```text
[稳定前缀] + [单一规范检查点] + [活跃尾部]
 system/core tools    canonical checkpoint   recent turns/unresolved tools
```

运行时正确性只依赖持久 transcript、tool artifact、单一检查点和当前内存快照；
provider cache、hash 与 observer 只能影响性能和诊断，不能成为恢复或压缩语义的权威。

本功能仍处于新功能阶段，不承担未发布实验结构的兼容成本。重构采用 clean break：删除
被替代路径和垃圾状态，不保留双写、旧读 fallback、影子实现或无限期 feature flag。

## 2. 当前需要收敛的问题

1. Cache observer 同时承担 wire 之前的 hash、bucket、break 分类、pending observation
   和推测式 hit/miss；其输入不是 provider 最终 wire，职责过重且结论不权威。
2. CORE 与 mounted DEFERRED 共用完整 tools hash，动态挂载会放大为整个工具集变化。
3. `session_compaction_history` 是追加摘要日志，不是有硬上限的当前状态；多份摘要重新
   注入会重复、冲突并随长任务增长。
4. 主动 compact 与 reactive compact 存在近似但不同的数据路径；溢出重试必须使用
   当前内存快照，不能退回较旧的持久 session。
5. 工具结果存在完整内存、静默截断、artifact preview、摘要等多种表示，发送、保存和
   恢复可能不一致。
6. 类型化预算分层适合测量，但不应继续演变为每层独立缓存和生命周期状态机。

## 3. 目标架构

### 3.1 稳定前缀

- 稳定前缀仅包含最终 provider wire 形态的 stable system 与 CORE tools。
- provider shaping 完成后计算一个 `stable_prefix_hash`；Agent 层不预测服务端命中。
- mounted DEFERRED 作为动态后缀，不进入稳定前缀身份。
- provider usage 中真实 cache read/create token 是命中事实来源；observer 降级为遥测。

### 3.2 单一规范检查点

- 每个 session 最多一个模型可见 `canonical_checkpoint`。
- 更新规则固定为“旧检查点 + 本次被裁剪的完整消息 → 新检查点”，成功后原子替换。
- 历史检查点可留在耐久 rollout/transcript 用于审计，但不得重新注入模型上下文。
- 检查点使用固定字段：目标与约束、已完成事项、关键决定、当前状态、未解决问题、
  下一步、必须保留的标识符和 artifact 引用。

### 3.3 活跃尾部与工具链

- 从最旧的完整 turn 边界裁剪，保留最近有界窗口。
- 已完成的历史工具调用链从活跃 prompt 删除；完整记录仍保留在 transcript/artifact。
- 未完成的 assistant tool call 与对应 tool result 必须成组保留，禁止裁成孤儿消息。
- reactive compact 和主动 compact 调用同一 `compact(current_snapshot, reason)` 入口；
  `reason` 只用于遥测，不改变压缩语义。

### 3.4 两级轻量压缩

一级机械压缩默认执行，不调用模型：

- 折叠已完成工具链；
- 大工具结果统一转为 artifact 引用和有界预览；
- 删除重复状态重宣告和低价值日志；
- 按完整 turn 边界裁剪最旧内容。

只有机械压缩后仍超过硬预算，或累计待归并内容达到阈值，才执行二级语义压缩，生成并
原子替换规范检查点。不得为每次轻微预算波动调用摘要模型。

## 4. 禁兼容与垃圾代码政策

以下为实施门禁，不是建议：

1. 禁止为 C1–C4 的未发布内部 metadata/schema 保留兼容读取器或迁移器；旧实验字段可
   在 session 加载时忽略，首次新写直接使用新结构。
2. 禁止新旧 compaction history 与 canonical checkpoint 双写、双读或合并注入。
3. 禁止保留 legacy compact/reactive compact 两套执行器；新入口验收后删除旧入口。
4. 禁止保留无 artifact 的静默工具结果截断 fallback；超限结果必须 artifact 化，失败
   时返回明确错误或明确有界降级状态。
5. 禁止为了比较实现而长期保留 feature flag、shadow path、dead enum variant、旧 hash
   ledger 字段和未消费 revision。短期迁移开关必须在同一切片删除后才能关闭切片。
6. 禁止用 adapter 前的本地结构 hash 冒充最终 wire hash；旧推测式 hit/miss 状态机在
   provider usage 接线完成后删除。
7. 删除公共接口前先确认没有已发布外部消费者；crate 内部实验接口直接 clean break，
   真正稳定的用户数据只保护 transcript、artifact 与用户配置。
8. 每个切片必须包含 deletion-proof 测试或 `rg`/编译门禁，证明旧符号和双路径已消失。

明确保留的 C1–C4 价值：typed section 顺序契约、CORE/DEFERRED 授权边界、
search-before-mount、artifact 隔离校验、Recall 顺序与 Drop 语义。保留行为契约，不保留
造成重复状态的具体内部实现。

## 5. 修订后的实施切片

### C5a：最终 wire 稳定前缀

- 把 hash 计算移动到 provider shaping 之后。
- 以 `stable_prefix_hash` 替代完整 tools/system/per-tool 运行时判断组合。
- provider cache namespace 由 adapter 提供；真实 usage 作为命中依据。
- 删除推测式 hit/miss 与不再消费的 bucket/pending 状态。

### C5b：单一检查点与统一压缩入口

- 引入有版本的 `canonical_checkpoint`，但不读取旧实验 compaction metadata 来兼容。
- 主动与 reactive compact 统一使用当前内存快照。
- 完成链机械折叠；未决工具组配对保留。
- 替换并删除模型可见的多摘要注入路径。

### C5c：工具结果表示统一

- 发送、保存、恢复统一使用小结果原文或 artifact ref + preview。
- 删除无 artifact 静默截断及重复 preview 生成路径。
- 保持 workspace/session 绑定、脱敏、容量、TTL、摘要校验和缺失语义。

### C5d：预算瘦身、恢复验收与删除证明

- 预算收敛为稳定前缀、检查点、活跃尾部三个顶层区域；保留必要的测量标签，不建立
  新的分层缓存状态机。
- 覆盖长任务、重启、compact 后续写、provider overflow、工具调用中断、artifact 缺失、
  Recall Drop、mounted 工具重宣告和 cache 完全失效场景。
- 执行旧符号/旧 metadata 写入/双路径 deletion-proof，更新 ADR 与运维指标说明。

### C5e：DEFERRED 工具自动激活

- 用户配置和 policy 只决定 Installed/Authorized；模型不修改长期授权，只管理当前任务的
  Active 工具集合。
- 保留 `tool_search`，搜索返回项自动进入下一次 provider call；删除模型可见
  `mount_tool`，不再要求“搜索 → 挂载 → 执行”三步协议。
- 将持久 `discovered`/`mounted`/revision 收敛为最多 8 个 task-local
  `active_deferred_tools`；新搜索替换旧集合，新 turn 回收未使用项。
- 未完成 assistant tool call 与 result 配对所需工具强制保留；provider retry 复用同一个
  active snapshot，不重复搜索或修改状态。
- 自动激活不等于批准执行：builtin、MCP server、mask、plan phase、read-only 和 approval
  仍在 Authorized/Executable 边界生效，registry rebuild 必须重新过滤。
- 采用 clean break，删除 `tool_discovery_v1`、`tool_not_discovered`、独立 mount revision
  和兼容恢复；仅在同 turn 中断恢复确有必要时持久化有界 active names。
- deletion-proof 和闭环测试覆盖 search → 下一次 call 直接执行、授权不可绕过、容量回收、
  重试幂等、工具组中断恢复和旧符号消失。

## 6. 验收指标

- 正确性：cache 完全禁用时，会话行为、恢复与工具配对仍一致。
- 有界性：模型可见检查点只有一个，活跃尾部和 artifact preview 都有硬上限。
- 一致性：同一工具结果在发送、保存、重启恢复后的结构语义一致。
- 轻量性：未越过二级阈值时不调用摘要模型；机械压缩可重复且幂等。
- 性能：只跟踪最终输入 token、cache read/create token、首 token 延迟、机械/语义压缩
  次数和恢复一致性失败；不以本地猜测命中率作为发布门。
- 清洁度：旧执行路径、双写、fallback、dead state 和临时 flag 均有删除证明。

## 7. 非目标

- 不改变 BML/Laputa、Recall 排序算法或工具授权策略。
- 不引入 TF-IDF、embedding、长期任务 soak 平台或新的 provider 缓存协议。
- 不为尚未发布的 C1–C4 实验 metadata 提供跨版本迁移承诺。
- 本文只修订技术计划；生产代码将在后续按 C5a–C5e 独立锁、独立提交实施。
