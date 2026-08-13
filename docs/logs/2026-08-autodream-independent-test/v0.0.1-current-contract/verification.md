# Verification

## 独立测试命令与结果

| 命令 | 结果 |
| --- | --- |
| `cargo test -p agent-diva-autodream -- --test-threads=1` | **56 passed**（16 lib + 40 既有 integration）；后补表征 3 条另跑 |
| `cargo test -p agent-diva-manager --test autodream_laputa_e2e -- --test-threads=1` | **6 passed** |
| `cargo test -p agent-diva-autodream --test current_contract` | **3 passed**（表征） |

未跑全仓 `just ci`：本轮不改生产逻辑，只加表征测试与决策文档。

## 测到了什么

现有套件覆盖：输入收集、候选闸门、默认 `MemoryPatch`/`Deprecation` 反射、
Laputa `create_proposal` 发出、worker 四阶段/锁/取消/超时、日报周报月报、
Manager HTTP 治理闭环（apply / deny / 抑制 / 编辑换绑 / 幂等 / 注入拒绝 /
provider 失败）。

## 测不到什么（诚实缺口）

- **STM 整理**：`agent-diva-autodream` 源码零 `STM`/`stm` 符号；产品 STM 未实现。
- **人格 Markdown 允许表**：没有 `IDENTITY.MD` 等提案类型，默认仍发 `MemoryPatch`。
- **真实 LLM 反射**与完整阶段日志：生产路径几乎只有 `worker.rs` 两条 `tracing::warn`。
- GUI / 真机桌面 AutoDream 未跑。

表征测试把上述缺口变成可重复断言，避免再次「不测就改口径」。
