# Verification

- Runbook 与 `docs/dev/autodream-laputa-product-closure/13-acceptance-criteria.md` 的原 G2D
  六场景和新增纵向第七场景一致。
- 覆盖 fresh/upgrade profile、候选 commit/hash、typed readiness、revision、治理关联 ID、
  GUI/日志证据与最终签署。
- 明确自动化不能冒充真机，真实 provider/密钥需用户另行授权。
- 文档链接、格式和 staged diff 在提交前检查。
- 未修改 Rust、Vue、Tauri、配置、数据库或用户 profile；因此 workspace 编译与 GUI
  smoke 不适用于本次文档切片。E7 自动化门沿用 commit `0c29c2a0` 的已通过记录。
