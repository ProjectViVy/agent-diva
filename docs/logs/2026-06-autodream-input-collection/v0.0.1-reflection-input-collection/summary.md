# Summary

## v0.0.1-reflection-input-collection

本轮完成 Story 3.2，给 `agent-diva-autodream` 增加了受限输入采集能力。实现按优先级读取最近会话、Laputa 只读 section，以及可选的 compact capsules，并在总预算内做截断与 omission 记录。

## Changes

- 新增 `agent-diva-autodream/src/inputs.rs`，实现 bounded input collector。
- 扩展 `AutoDreamRunRecord`，增加 `input_summary` 结构化摘要字段。
- 在 `AutoDreamService` 中增加 `collect_inputs`，将采集摘要写回当前 run record。
- 补充 collector/service/output 相关测试，确保 crate 级回归通过。

## Impact

- AutoDream 现在可以在不直接写 `.laputa`、`MEMORY.md` 或 Mentle 的前提下，为后续反思阶段准备受限上下文。
- 当前交付仅覆盖输入采集与 run metadata，不包含 3.3 的 restricted prompt 执行逻辑。
