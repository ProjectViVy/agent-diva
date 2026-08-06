# Acceptance — GA-MEM-PARITY Wave 6

## 代码级验收（自动化）

| 验收项 | 通过条件 | 状态 |
|--------|---------|------|
| S1 memory_list 不返回 superseded 记录 | wave5_acceptance 测试断言 NOT-in-list | ✅ |
| S2 同会话 memory_add 后 startup 即时可见 | f4 测试不加 drop/open 即可读到 | ✅ |
| S3 evidence_refs 空时 advisory 非空 | wave6_tests 两个用例 | ✅ |
| 全 workspace clippy -D warnings（S3 crates） | 0 warnings | ✅ |
| 全 workspace 测试（S3 crates） | 全绿 | ✅ |

## 产品级验收（归 G2D+）

以下验收项需要真机桌面环境（GUI + CLI 联通），不在本 Wave 自动化范围：

- G1 手动触发端到端（memory_add → 下次会话 startup 可见）
- G2 AutoDream 手动触发 → 候选 → 提案 → 审批 → 权威
- G3 tombstone GUI 可见性与过滤
- G5/G6 审查 UI / 节律报告
- G7 真机节律验证
- G10–G12 真机端到端

## 延期项

- B9 完整 tool-result 强制校验（需 agent_loop evidence 链跟踪）
- `agent-diva-manager` `superseded_memory_digests` pre-existing 编译错误
