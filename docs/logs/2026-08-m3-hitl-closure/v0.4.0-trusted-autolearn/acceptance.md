# Acceptance

## 用户/产品视角验收步骤

1. GUI「信任」模式：放行一条未知命令（如 `curl https://example.com`）。
2. 该命令被自动学习为 Allow 规则，落盘到 coordinator 规则文件同目录的
   `execpolicy-guardian.toml`。
3. 同会话后续再执行该命令命中 known-safe，不再询问。
4. 重启后从 guardian 文件装载，仍命中 known-safe。

## 验收通过标准

- sandbox 127 / tools 117 测试 + clippy 严格模式全绿。
- 学到的规则不覆盖 coordinator 的 execpolicy.toml（独立文件）。