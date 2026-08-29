# CHANNEL-EPIC C1 验证记录

## 合同与定向验证

- 共享 JSON Schema 和 fixture 均可解析；Rust `jsonschema` 与 TypeScript AJV 使用同一份根 schema。
- `cargo test -p agent-diva-core --test channel_protocol_tck`：7 passed。
- `cargo test -p agent-diva-channels --test channel_characterization`：5 passed。
- `pnpm exec vitest run src/protocol/neuro-link-v1.test.ts`：16 passed。
- `cargo bench -p agent-diva-core --bench channel_capacity -- --noplot`：五个固定 lane
  均完成采样并返回成功结果。

## 标准门禁

- `just fmt-check`：通过。
- `just check`：通过，workspace clippy `-D warnings`；仅有既有 `imap-proto` future-incompatibility 提示。
- `just test`：通过，workspace 单元、集成和 doc tests 全部通过；新增 Rust TCK 在全量运行中为
  7 passed。既有测试中有明确标记的 ignored performance/health cases，保持原状。
- `pnpm test`（`agent-diva-gui`）：74 files、532 tests passed。
- `pnpm build`（`agent-diva-gui`）：`vue-tsc --noEmit` 和 Vite production build 通过；仅有既有
  chunk size warning。
- `pnpm install --frozen-lockfile --ignore-scripts`：通过。

## 已知提示

`npm install --package-lock-only --ignore-scripts` 在同步已跟踪的 npm lock 时报告依赖图有 11 个
audit vulnerabilities（2 moderate、9 high）；本批未运行 `audit fix`，已在 `TODOLIST.md` 单独登记。
这不改变 pnpm lock、协议 schema 或运行时行为。
