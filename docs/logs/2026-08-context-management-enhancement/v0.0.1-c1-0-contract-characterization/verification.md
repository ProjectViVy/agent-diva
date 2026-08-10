# 验证

## 自动化

- `cargo test -p agent-diva-agent c1_0`：3 passed。
- `cargo test -p agent-diva-agent context_assembly`：3 passed。
- `cargo test -p agent-diva-agent --lib`：390 passed。
- `cargo clippy -p agent-diva-agent --lib -- -D warnings`：通过。
- `just fmt-check`：通过。
- `just check`：通过。
- `just test`：通过。

首次 `just test` 因执行工具的 120 秒时限被终止，并导致测试输出管道 BrokenPipe；
使用 360 秒时限完整重跑后全绿，未发现用例失败。

## 行为检查

- 本切片未将 `ContextBuilder` 迁移到新 assembler，生产消息布局不变。
- 当前 tool definitions 只刻画稳定集合，不宣称已有稳定顺序；稳定排序属于 C1。
- 本切片没有用户可见行为变化，因此无需 CLI/GUI smoke。
