# Release

## Release Type

- 内部研发基线发布（CI 方案与文档）

## Deployment Method

- 不执行二进制发布。
- 当前版本仅将多平台构建矩阵与 artifact 生成方式固化到仓库内，供下一阶段 `CA-CI-ARTIFACTS` 直接接入 Release 流程。

## Follow-up Release Suggestion

- 在下一阶段基于当前 artifact 命名规范接入：
  - `actions/download-artifact`
  - `softprops/action-gh-release`
- 在发布前补齐：
  - Release workflow 与真实二进制命名的对齐
  - GUI / Headless smoke job
  - 分发包随附的完整 README 与服务模板
