# 验证

已通过：

- `cargo test -p agent-diva-autodream`
  - 46 个测试通过。
  - 覆盖真实类型候选、证据真实性、compaction-only、workspace 隔离、重复、直接
    矛盾、容量、prompt injection、敏感内容、PII 脱敏、provider unavailable、
    no-candidates 和 proposal artifact 元数据。
- `cargo test -p agent-diva-manager reflection_adapter --lib`
  - 验证 provider adapter 解析 fenced JSON、候选 ID 归一化及
    `deepseek-chat` 原始 model ID 不改写。
- `cargo test -p agent-diva-manager autodream --lib`
  - queued 后台失败/成功发布路径通过。
- `cargo clippy -p agent-diva-core -p agent-diva-autodream -p agent-diva-manager -- -D warnings`
  - 通过。

提交前继续执行 `just fmt-check` 与 `just check`。完整 `just test` 的 Windows 桌面
占用和 GUI PDB 问题仍由 E7 发布门统一复验。本切片不调用真实外部 API，也不读取
桌面密钥，不执行人工桌面测试。
