# Harness Gap Diva 化适配：迭代总结

## 交付范围

本迭代完成一次当前状态调研和研究方案，不修改 Rust、Tauri、配置或构建文件。

## 主要结论

1. 旧报告中 Plan Mode、流式 Provider、Prompt Injection、Approval 的缺口判断已经过时，
   必须以当前 crate 代码为准。
2. 当前最值得独立研究的两个缺口是：
   - 可组合但受限的 Rust Trait Hook Kernel；
   - per-session 有界串行 admission/backpressure。
3. OpenHarness、ZeroClaw、OpenFang、Claude Code 的机制可以作为证据，但不能把它们的
   Python/HTTP/LLM hook、WASM 插件、全局 event OS、remote control 或 permission mode
   直接搬入 Diva。
4. 方案保持 BML、Persona、ACTMEM、Evolution、Sandbox、Approval 的既有权威边界。

## 文档交付

- `docs/research/harness-gap-diva-adaptation-2026-08/README.md`
- `current-state-and-evidence.md`
- `reference-comparison.md`
- `diva-adaptation-proposal.md`
- `docs/research/README.md` 入口更新
- `TODOLIST.md` 增加 session admission 条目并记录 Hook 适配边界
