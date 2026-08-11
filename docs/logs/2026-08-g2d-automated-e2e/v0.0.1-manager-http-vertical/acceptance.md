# G2D Automated E2E Acceptance

## 自动化验收

在仓库根目录执行：

```text
cargo test -p agent-diva-manager --test autodream_laputa_e2e
just ci
```

预期：新增套件 6/6 通过，`just ci` 全部通过。

## 用户产品验收边界

自动化通过后，G2D+ 仍需在真实桌面逐项确认批准、拒绝、编辑后批准、重复点击、重启
恢复、回滚，以及“真实任务 → evidence → AutoDream → proposal → apply → 新会话 Recall
→ rollback 后消失”第七旅程。
