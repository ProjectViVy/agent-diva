# Acceptance

1. 21 条 `partial` 每条都有固定 Octos SHA 的 source path/symbol/anchor 与当前 DIVA
   symbol 对照。
2. 每条都明确记录 fixture、测试源码、精确 request/response 或 opcode、receipt/error、
   timeout/cancel/retry/dedup/health/reconnect 证据和缺口。
3. `verified` 只在已有完整实现和既有通过记录满足清单规则时使用；否则保持 `partial`。
4. 无法由平台或官方协议证明的能力，记录为明确的 `blocked/unsupported`，并证明首次
   transport 调用前返回 typed error；不得静默降级或猜测。
5. Gate3 页面、`evidence-manifest.md`、`capability-evidence.json` 三处状态一致；
   不修改冻结 capability matrix。
6. 本轮无编译、无测试执行、无外网/live smoke；最终只做文档静态一致性与差异检查。
