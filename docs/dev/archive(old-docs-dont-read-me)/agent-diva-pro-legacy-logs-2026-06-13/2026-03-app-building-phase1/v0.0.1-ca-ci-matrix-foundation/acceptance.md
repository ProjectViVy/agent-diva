# Acceptance

## Product Acceptance Steps

1. 打开 `docs/app-building/README.md`，确认阶段建议已更新为“`CA-HL-CLI-GATEWAY` 已完成，当前优先 `CA-CI-MATRIX`”。
2. 打开 `docs/app-building/wbs-ci-cd-and-automation.md`，确认 `WP-CI-MATRIX-01/02/03` 已具备控制账户边界、技术路线、代码级 workflow 片段与验收门禁。
3. 打开 `.github/workflows/ci.yml`，确认存在三平台 `rust-check`、`gui-build`、`headless-build` job。
4. 打开 `scripts/ci/package_headless.py`，确认 Headless 压缩包命名规则与随包 README 逻辑已固化。
5. 打开 `docs/app-building/headless-bundle-quickstart.md`，确认 Headless artifact 至少具备可下载、解压、启动的最小运行说明。

## Acceptance Result

- 当前版本满足第一阶段 `CA-CI-MATRIX` 的基线交付，可作为下一阶段分发、发布与 smoke 测试的输入。
