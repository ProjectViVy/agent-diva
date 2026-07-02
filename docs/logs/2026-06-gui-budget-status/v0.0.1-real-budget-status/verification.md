# Verification

- `cargo check -p agent-diva-manager -p agent-diva-gui`
  - 结果：通过。
- `pnpm -C agent-diva-gui build`
  - 结果：通过，`vue-tsc --noEmit` 与 `vite build` 均成功。
- `pnpm -C agent-diva-gui test -- src/utils/contextBudget.test.ts`
  - 结果：通过，2/2 用例成功。
- `cargo test -p agent-diva-manager --lib`
  - 结果：通过，27/27 用例成功。
- `cargo fmt --all --check`
  - 结果：失败，受既有 `agent-diva-core/src/lib.rs` 格式漂移阻塞，已记录到 `TODOLIST.md`。
- `cargo test -p agent-diva-gui --lib`
  - 结果：失败，既有测试 `embedded_server::tests::embedded_gateway_serves_health_endpoint` 返回 `502`，已记录到 `TODOLIST.md`。
