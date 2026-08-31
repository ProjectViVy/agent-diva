# Verification

## Planned gates

1. 六份频道深扫报告均包含 DIVA/Octos source anchor、外部操作、权限、媒体、生命周期、决策和 fixture。
2. `endpoint-ledger.md` 与六份报告的 endpoint/function 交叉引用一致。
3. `cross-cutting-gap-matrix.md` 覆盖图片识别、群聊、审批、权限，且不把 legacy partial 写成 complete。
4. `evidence-manifest.md` 的 planned/blocked 状态与 capability matrix 一致。
5. 所有相对 Markdown 链接、表格结构和脱敏规则通过文档检查。
6. 按项目规则运行 `just fmt-check && just check && just test`；文档-only 也记录结果。

## Result

- Markdown relative-link checker：通过，C5 package 29 个 Markdown 文件均可解析。
- Decision queue checker：通过，`decision-requests.md` 的 Open 区为 `None.`。
- `git diff --check`：通过。
- `just fmt-check`：通过（退出码 0）。
- `just check`：通过（退出码 0）；Cargo 报告既有 `imap-proto v0.10.2` future-incompatibility 提示。
- 首轮 `just test`：退出码 1；432 个测试通过，既有
  `agent_loop::tests::concurrent_sessions_keep_provider_retry_correlation_isolated` 超时失败。
  该项已加入 `TODOLIST.md` 的 `AGENT-RETRY-CORRELATION-FLAKE`，未在本迭代修改产品代码。
- 聚焦重跑该测试 3 次：全部通过。
- 第二轮全量 `just test`：退出码 0；workspace 单元测试、集成测试和 doc-tests 全部通过。

结论：C5-P2 文档交接门禁通过，第二轮 workspace 全量测试通过；首轮暴露的并发偶发性仍保留在
`TODOLIST.md`，后续应继续稳定性治理，不因一次重跑通过而删除记录，也不关闭 C5-I/C5-V/C6。
