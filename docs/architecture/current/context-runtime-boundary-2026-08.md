# 上下文运行时当前边界

- 状态：`C1–C5 Implemented / Current Runtime Contract`
- 日期：2026-08-13
- 详细依据：[Context Management Enhancement](../../research/context-management-enhancement-2026-08/README.md)

## 当前运行时形态

```text
[stable system + CORE tools]
        + [canonical checkpoint]
        + [active tail / unresolved tool groups]
```

- 稳定前缀以最终 provider wire 形态为准；模型、provider、策略变化必须可解释地造成
  cache break。
- `canonical_checkpoint_v1` 是当前压缩/恢复的单一检查点，不与旧摘要日志并存为第二真源。
- 工具结果使用受保留策略约束的 artifact/reference 表示；引用不能伪造、越权或绕过配对校验。
- DEFERRED 工具按需发现并在同一 turn 的下一次 provider call 生效；装配仍受 mask/plan
  policy 约束。
- provider cache 命中、hash 和 observer 只属于性能/诊断遥测，不是恢复或压缩语义的权威。

## 与 STM / ACTMEM 的关系

当前 session `working_memory`、session transcript、canonical checkpoint 和跨会话 ACTMEM
必须保持不同生命周期。不证明 ACTMEM 已经实现。

产品（2026-08-14）：ACTMEM **正文不进**稳定前缀。日常 CORE 只允许一个查询工具
`actmem`；管理工具必须划进已有 DEFERRED，经 `tool_search` 激活。不得为 ACTMEM
另做一套挂载机制。

## 证据

- C1–C5 的实施、测试和 clean-break 证据保留在 `docs/logs/2026-08-context-management-enhancement/`。
- 未来 STM 的物理权威、Layer 1 装配、预算、并发和失败恢复仍以 STM 专项研究为准。
