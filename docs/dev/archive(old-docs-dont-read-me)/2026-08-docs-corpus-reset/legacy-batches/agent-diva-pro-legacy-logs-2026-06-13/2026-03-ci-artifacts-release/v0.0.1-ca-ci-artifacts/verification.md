---
title: CA-CI-ARTIFACTS v0.0.1 验证记录
---

## 验证范围

- 验证对象：
  - 文档：`wbs-ci-cd-and-automation.md` 中的 `CA-CI-ARTIFACTS` 章节与相关 WP；
  - 文档：`wbs-distribution-and-installers.md` 第 4 章、`wbs-validation-and-qa.md` 中的 `WP-QA-REG-03`；
  - Workflow：`.github/workflows/release-artifacts.yml`。
- 验证目标：
  - 确认文档与 workflow 在命名、触发条件与资产流向上的描述一致；
  - 为后续在真实仓库环境中执行 `WP-QA-REG-03` 提供“预期行为”基线。

## 静态验证（本地 / 代码级）

- **YAML 静态检查**
  - 逐行审阅 `release-artifacts.yml`，确认：
    - `on.workflow_run` 监听的 workflow 名称与 `ci.yml` 中的 `name: CI` 对齐；
    - `jobs.release.if` 条件能够过滤出“CI 成功且 head_branch 以 `v` 开头”的场景；
    - `actions/download-artifact@v4` 使用 `run-id: ${{ github.event.workflow_run.id }}`（通过 `meta` 步骤输出）；
    - 归一化脚本仅依赖 Bash + 标准 tar/unzip 命令，适合在 `ubuntu-latest` 上运行；
    - `softprops/action-gh-release@v2` 使用 `GITHUB_TOKEN`，具备 `contents: write` 权限。
- **文档一致性检查**
  - 将以下文档中的描述进行交叉比对：
    - `wbs-ci-cd-and-automation.md` 中对 `CA-CI-ARTIFACTS` 的 CA/WP 与触发策略描述；
    - `wbs-distribution-and-installers.md` 第 4 章中对 Release 资产来源与映射的说明；
    - `wbs-validation-and-qa.md` 中 `WP-QA-REG-03` 的 Release 验收 checklist。
  - 结论：术语与文件路径一致，均指向 `.github/workflows/release-artifacts.yml` 及 GitHub Releases 资产。

> 说明：当前环境下未直接在远程 GitHub 仓库中触发 workflow 运行，故本轮验证仅限于“静态结构与逻辑对齐”，不包含真实 CI/CD 执行结果。

## 建议的在线验证步骤（待执行）

> 以下步骤应在首次采用该方案进行真实版本发布时执行，并将结果补充回本文件。

1. **准备条件**
   - 确保 `main` / `develop` 分支上的 `CI` workflow 处于绿色状态；
   - 选定一个版本号（例如 `v0.2.0`），并确认 `agent-diva-cli/Cargo.toml` 中的 `package.version` 已与该版本对齐。
2. **触发 CI 与 Release**
   - 向远程仓库推送 tag：`git tag v0.2.0 && git push origin v0.2.0`；
   - 等待 `CI` workflow 在该 tag 上运行完成且成功；
   - 确认 `Release Artifacts` workflow 被 `workflow_run` 事件触发并成功执行。
3. **检查 Release 页面**
   - 打开 GitHub Releases 中的 `v0.2.0` 版本；
   - 按 `WP-QA-REG-03` 中的 checklist 核对：
     - Assets 中是否包含三平台 GUI 安装包与 Headless 压缩包；
     - 资产命名是否与 `wbs-distribution-and-installers.md` / `wbs-headless-cli-package.md` 描述一致。
4. **执行最小 smoke**
   - 从 Release Assets 中选择：
     - 1 个 GUI 安装包（例如 Windows NSIS 或 macOS `.dmg`），按 GUI smoke WBS 执行；
     - 1 个 Headless 压缩包（例如 Linux `tar.gz` 或 Windows `.zip`），按 Headless WBS 执行；
   - 将执行结果与发现的问题记录回本文件。

## 当前结论

- 文档与 workflow 设计已经满足本轮的“结构性与可执行性”要求，可作为 `CA-CI-ARTIFACTS v0.0.1` 的实现基线；
- 需要在首次正式发布时补充一轮“在线执行验证”记录，以完成 `WP-QA-REG-03` 的闭环。

