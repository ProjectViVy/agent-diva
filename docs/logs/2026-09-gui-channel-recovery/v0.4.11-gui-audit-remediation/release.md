# GUI dependency audit remediation release

日期：2026-09-04

本轮只在本地 `feat/gui-channel-recovery` 隔离分支提交，没有 push、发布安装包或修改远程
环境。产品提交为：

- `42828d22 fix(gui): remediate frontend dependency advisories`
- `82c834b5 chore(gui): enforce pnpm audit baseline`

合入后，开发与打包环境需要通过 Corepack 使用仓库固定的 pnpm 版本；不再支持从
`package-lock.json` 恢复 npm 依赖图。回滚上述提交会重新引入双 lockfile、旧依赖图和缺失
持续审计门禁，因此只能作为紧急源码回滚，不能视为安全等价版本。
