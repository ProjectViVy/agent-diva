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

## 与 STM 的关系

当前 session `working_memory`、session transcript、canonical checkpoint 和未来跨会话 STM
必须保持不同生命周期。C1–C5 的实现是 R2 输入；R2 研究包见
[`../../research/cognitive-r2-stm-context-2026-08/README.md`](../../research/cognitive-r2-stm-context-2026-08/README.md)。
不证明 STM 已经实现，也不授权把现有 checkpoint 直接改名为 STM。

## 证据

- C1–C5 的实施、测试和 clean-break 证据保留在 `docs/logs/2026-08-context-management-enhancement/`。
- 未来 STM 的物理权威、Layer 1 装配、预算、并发和失败恢复仍以 STM 专项研究为准。
