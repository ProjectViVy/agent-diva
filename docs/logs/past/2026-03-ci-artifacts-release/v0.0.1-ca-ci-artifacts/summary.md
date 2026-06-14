---
title: CA-CI-ARTIFACTS v0.0.1 实施总结
---

## 背景

- 目标：在不修改核心业务代码的前提下，为 Agent Diva 引入 `CA-CI-ARTIFACTS`，将现有 `CA-CI-MATRIX` 产出的三平台 GUI / Headless artifacts 固化为可发布的 Release 资产。
- 范围：仅涉及 CI/CD workflow 与构建文档（WBS / README / logs），不改变运行时行为与对外接口。

## 本轮主要变更

- **CI/CD WBS 更新**
  - 在 `docs/app-building/wbs-ci-cd-and-automation.md` 中新增 `CA-CI-ARTIFACTS` 章节及以下 WP：
    - `WP-CI-ART-01`：Release 触发与版本/tag 策略；
    - `WP-CI-ART-02`：从 CI 下载并归一化 GUI / Headless artifacts；
    - `WP-CI-ART-03`：使用 GitHub Releases 发布 `dist/gui/**` 与 `dist/headless/**` 资产。
- **Release workflow 新增**
  - 新增 `.github/workflows/release-artifacts.yml`：
    - 通过 `workflow_run` 事件监听 `CI` workflow 的成功运行；
    - 支持 `workflow_dispatch` 手动指定 `run_id` + `tag` 进行补发；
    - 使用 `actions/download-artifact@v4` 从 CI run 下载 artifacts 到 `_artifacts/`；
    - 规范化整理为 `dist/gui/**` 与 `dist/headless/**`；
    - 校验 Headless 包内至少包含 `agent-diva(.exe)`、`README`、`bundle-manifest`；
    - 使用 `softprops/action-gh-release@v2` 基于 tag 创建/更新 Release 并上传资产。
- **分发与 QA 文档对齐**
  - 在 `wbs-distribution-and-installers.md` 中新增第 4 章，明确：
    - GUI 安装器与 Headless 压缩包的“官方获取方式”为 `Release Artifacts` workflow 生成的 GitHub Releases 资产；
    - 将 Release 资产映射回 `CA-DIST-GUI-INSTALLER` / `CA-DIST-CLI-PACKAGE` 的 CA / WP 场景。
  - 在 `wbs-validation-and-qa.md` 中新增 `WP-QA-REG-03`：
    - 定义 Release 级别的验收 checklist（资产完备性 + 至少一条 GUI smoke + 一条 Headless smoke）。
- **总览文档更新**
  - 在 `docs/app-building/README.md` 的“阶段建议”中补充阶段 4 的现状，标记 `CA-CI-ARTIFACTS` 第一版基线已落地，并说明后续与 QA smoke CA 的衔接。

## 影响范围

- 对人类开发者/运维：
  - 新增一个可复用的 Release workflow（`Release Artifacts`）以及与之配套的 WBS 说明；
  - 发布流程从“手工拼装资产”升级为“通过 CI artifacts 自动聚合并发布”。
- 对 Agent / 子 Agent：
  - 在后续迭代中，可以以 `CA-CI-ARTIFACTS` 为上游，直接消费 Release 资产执行分发验证与 QA smoke。
- 对运行时行为：
  - 本轮不修改任何 crate 的业务逻辑与对外接口，不改变运行时路径，仅新增 CI/CD 层能力。

