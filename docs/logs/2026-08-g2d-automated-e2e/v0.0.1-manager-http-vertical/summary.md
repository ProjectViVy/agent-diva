# G2D Automated E2E Summary

## 完成内容

- 新增 `agent-diva-manager/tests/autodream_laputa_e2e.rs` 独立 HTTP 集成测试套件。
- 通过 Manager Router、AutoDream Service、Laputa governance 与 typed BML 的真实生产
  seam 运行 deterministic provider，不读取外部密钥。
- 覆盖 6 个可重复场景：
  1. evidence → AutoDream → proposal → approval → typed apply → Recall feedback → rollback；
  2. rejection suppression；
  3. proposal edit 后撤销旧 governance request；
  4. apply replay 幂等；
  5. provider unavailable fail-closed；
  6. prompt-injection candidate gate 拒绝。

## 边界

该批次证明自动化纵向链路，不替代 G2D+ 的真实桌面、真实 provider 与人工操作验收。
