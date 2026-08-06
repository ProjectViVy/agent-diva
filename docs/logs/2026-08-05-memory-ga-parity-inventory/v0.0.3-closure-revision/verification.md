# Verification — GA-MEM-PARITY 闭环断点修订

## 验证方法

- 对照 GA 参考实现（`.workspace/GenericAgent`）逐环核对闭环六环：
  写（distill/start_long_term_update）→ 存（L2/L3 文件）→ 召回（global memory
  注入 + file_read）→ 遗忘（清理 SOP）→ 治理（SOP 纪律）→ 蒸馏（L4 12h 归档
  + salient mining）。
- 对 Diva 设计逐断点核对：
  - G1：确认 working checkpoint（域 C）在 Wave 2，而 distill 在 Wave 1 → 依赖倒挂成立。
  - G2：确认 D2/D3/D4（prefetch 生产可用）无 Wave 归属，W1-4「下次会话可见」
    承诺依赖该未排期能力 → 断点成立。
  - G3：确认 tombstone 模型存在但注入侧过滤无归属，acceptance 1.1「忘掉 X 后
    不再出现」需要注入侧配合 → 断点成立。
  - G4：确认 H5/G10 未产品化，AutoDream 候选与即时 apply 无冲突策略 → 断点成立。
- 修订均以文档形式落入 inventory §10.3 与 TODOLIST，无行为变更。

## 结果

- 4 断点全部记录并给出 Wave 归属修订（G1/G3→Wave 1，G2→Wave 3，G4→Wave 4）。
- 无行为变更（docs only），Rust/GUI 测试不适用。

## 遗留

- Wave 实施时需按 §10.3 验收 U1/U2/U3 六环闭环（以 G2/G3 为通过前提）。
