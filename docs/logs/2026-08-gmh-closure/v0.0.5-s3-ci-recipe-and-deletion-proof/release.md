# Release

本 slice 为 CI runner 配置与 CI 校验脚本的变更，无独立发布物。

## 部署方式

- CI 变更随仓库 push 自动生效（`.github/workflows/ci.yml`）。
- `scripts/ci/check_laputa_clean_break.py` 随 `just laputa-clean-break-check` /
  `just ci` 调用，无需部署。

## 说明

- 无停机、无迁移。
- 修复的是 CI workflow 对不存在 recipe 的调用，不改变 CI gate 严格度。