# 测试策略

## 1. 测试金字塔

### 单元测试

- evidence 脱敏、digest、容量、retention、workspace 隔离。
- run 每个合法/非法 transition。
- reflection schema：空候选、未知类型、无 evidence、越界 confidence。
- candidate duplicate、contradiction、suppression、sensitivity 与 risk 分类。
- proposal deterministic ID、编辑后 digest/version 变化。
- Recall feedback 不含 payload。

### 集成测试

- deterministic fake provider：session evidence → 2 个候选 → 2 个 proposal。
- provider timeout/invalid JSON/partial stream 不产生 proposal。
- restart 在每个 stage 边界恢复，不重复提案。
- approve/edit/reject/expire/revoke/concurrent/replay。
- typed apply 后 FTS 可检索；rollback 后不可见；tombstone/supersede 正确。
- workspace mismatch、store corruption、FTS failure 均 fail closed/degraded 明确。

### 纵向 E2E

1. 执行一个可验证的本地任务。
2. journal 记录脱敏结果证据。
3. 手动触发 AutoDream。
4. fake/real provider 生成结构化候选。
5. GUI/Manager 显示 proposal 与证据。
6. 批准并应用。
7. 新会话 Recall 命中该记录。
8. 回滚后 Recall 不再返回。

## 2. 崩溃窗口

覆盖 journal 写前/写后、reflection 前/后、proposal 第 N 条发布后、receipt 提交后、typed commit 后、feedback 写前，以及响应返回前崩溃。每个窗口必须证明“最多一次 authority 变化、可重放原结果、无孤儿状态”。

## 3. GUI 测试

- component tests：所有 run 状态、空状态、degraded、取消和 retry。
- API contract tests：typed reason code 与事件去重。
- Playwright/Tauri 最小 smoke：从触发到 proposal inbox。
- 最终真实桌面：原 G2D 六场景加完整纵向第七场景。

## 4. Gate

每切片：聚焦测试、`just fmt-check`、`just check`、`just test`。涉及 GUI 时追加 GUI tests/build 与 Tauri check；最终追加 deletion-proof、Rust 1.80 独立 target、10k Memory、长会话与并发恢复基线。

真实外部 API 只在最终 smoke 使用，必须由用户明确授权；此前所有 E2E 使用 deterministic fake，不读取 `keys.txt`。
