# GUI dependency audit remediation summary

日期：2026-09-04

## 结果

GUI 依赖管理已收敛为 pnpm 单一权威，并修复本轮审计命中的已知依赖漏洞。交付提交为
`42828d22` 与 `82c834b5`。

- `agent-diva-gui/package-lock.json` 已删除并加入忽略规则，仓库只保留
  `pnpm-lock.yaml`。
- package manager 固定为带完整 integrity hash 的 `pnpm 10.34.5`。
- `postcss` 升至 `8.5.23`，并用 pnpm override 固定
  `brace-expansion 2.1.4`、`browserslist 4.28.7`、`nanoid 3.3.18`、
  `postcss 8.5.23`。
- CI、`just`、Windows/macOS 打包脚本、GUI 冒烟脚本和文档全部改用 frozen pnpm
  安装，不再调用 npm、npx 或 yarn。
- CI 新增独立 `gui-audit` job，以 moderate 为最低失败级别，并成为 release job 的
  前置依赖；`scripts/ci/check_gui_dependency_policy.py` 阻止第二 lockfile、非 pnpm
  命令或审计门禁回退。

这次变更不修改 GUI 产品行为，也没有使用自动 `audit fix` 做不受控升级。
