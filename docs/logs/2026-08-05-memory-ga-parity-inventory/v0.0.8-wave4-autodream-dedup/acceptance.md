# Acceptance — GA-MEM-PARITY Wave 4（AutoDream 去重 — G4）

## 验收步骤

1. **服务方法契约**：`cargo test -p agent-diva-laputa wave4` 3 个
   `service::wave4_tests::*` 全部通过——`applied_authority_digests`
   返回 AppliedAuthority 活跃记录 digest，过滤 tombstone / session-scoped
   / supersedes 目标；空 workspace 返回空数组。
2. **端到端去重契约**：`cargo test -p agent-diva-autodream wave4` 3 个
   `worker::wave4_tests::*` 全部通过——typed authority 记录 → gate 拒绝
   同内容候选（Duplicate）；新内容通过；supersedes tombstone 后原内容可
   重新被接受。
3. **失败降级契约**：`applied_authority_digests_graceful_missing_store`
   覆盖；worker 侧降级为 `tracing::warn!` + 空数组（代码可见，无独立测试；
   Wave 5 可补注入式测试）。
4. **契约文档追溯**：`docs/architecture/memory-write-paths-contract.md`
   priority rule #1 含 "Realized in Wave 4" 注脚，指向具体 service 方法 +
   worker 双路接线 + CandidateGate Duplicate 拒绝。
5. **TODOLIST 状态**：GA-MEM-PARITY 顶部状态行显示
   "Wave 0/1/2/3/4 已完成"；新增 `WAVE4-AUTODREAM-G4` 子块（3 项已勾选）
   与 "Wave 4 延期项" 条目块（G1/G2/G3/G5/G6/G7/G10/G11/G12 归 G2D+
   或后续独立 Wave）。
6. **回归**：`cargo test --workspace` 全绿；CLI 6 个既有 wiremock 502
   失败（`CLI-WIREMOCK-502-PREEXISTING`）与本迭代无关。

## 验收结果

- [ ] 用户确认 6 项验收通过
- [ ] 用户确认可排期 Wave 5（巩固与清理：E4/E5 consolidation 条目化 +
      B7 GC 提案 + F3/F4/F6/F7 延期项收口）或 G2D+ 真机桌面验收

（由用户填写）
