---
title: CA-CI-ARTIFACTS v0.0.1 发布说明
---

## 适用范围

- 适用于基于 `CA-CI-MATRIX` 已经稳定生成三平台 GUI / Headless artifacts 的版本；
- 假设仓库中已存在以下 workflow：
  - `CI`（`.github/workflows/ci.yml`）；
  - `Release Artifacts`（`.github/workflows/release-artifacts.yml`）；
  - 以及原有的 Rust crate 发布流程（`.github/workflows/release.yml`）。

## 推荐发布流程（高层视角）

1. **准备版本号**
   - 在本地确定目标版本号（例如 `v0.2.0`）；
   - 确认 `agent-diva-cli/Cargo.toml` 中的 `package.version` 与该版本一致；
   - 如有需要，同步更新其他 crate 的版本并跑通 `just fmt-check && just check && just test`。

2. **创建并推送 tag**
   - 在经过代码评审与 CI 绿灯后，创建 tag 并推送到远程：

     ```bash
     git tag v0.2.0
     git push origin v0.2.0
     ```

   - 或在必要时通过仓库 UI / Release 界面创建 tag（需确保与代码版本一致）。

3. **等待 CI 与 Release workflow 完成**
   - 在 GitHub Actions 中观察：
     - `CI` workflow 在 tag 上运行并成功完成（包括 `rust-check`、`gui-build`、`headless-build` 等 job）；
     - `Release Artifacts` workflow 通过 `workflow_run` 事件被触发，成功完成 artifacts 下载、归一化与 Release 创建。

4. **校验 Release 资产**
   - 打开对应 tag 的 Release 页面，核对：
     - Assets 中包含三平台 GUI 安装包与 Headless 压缩包；
     - 资产命名与 `wbs-distribution-and-installers.md` / `wbs-headless-cli-package.md` 中说明一致。

5. **执行 Release 级 smoke 验收**
   - 按 `wbs-validation-and-qa.md` 中的 `WP-QA-REG-03` 执行一次 Release 级 smoke：
     - 选择 1 个 GUI 安装包 + 1 个 Headless 包进行安装 / 启动 / 基本功能验证；
     - 在本目录下的 `acceptance.md` 中记录验收结论。

## 与 crate 发布（crates.io）的关系

- `release.yml` 中的 `publish-crate` job 负责将 Rust crate 发布到 crates.io，与本次 `CA-CI-ARTIFACTS` 关注的 **应用级发行包**（GUI / Headless 安装包）相互独立但可协同：
  - 建议在同一版本 tag 下，将 crates.io 发布与 Release 资产生成作为一个发布波次的一部分；
  - 对外文档中应明确区分：
    - “crate 版本号”（面向 Rust 依赖方）；
    - “应用发行包版本号 + 安装包下载地址”（面向终端用户 / 运维）。

## 回滚与重发建议

- 如发现 Release 资产缺失或不符合预期：
  - 可以通过 `workflow_dispatch` 方式重新运行 `Release Artifacts` workflow，指定：
    - `run_id`：指向某一次成功的 `CI` 运行；
    - `tag`：保持与原 Release 相同；
  - 或在必要时删除有问题的 Release / tag，修复 CI 状态后重新打 tag 发布。

