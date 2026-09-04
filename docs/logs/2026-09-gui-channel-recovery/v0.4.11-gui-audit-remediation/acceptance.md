# GUI dependency audit remediation acceptance

日期：2026-09-04

## 验收步骤

1. 在仓库根目录运行 `python scripts/ci/check_gui_dependency_policy.py`，应返回通过。
2. 进入 `agent-diva-gui`，运行 `corepack pnpm install --frozen-lockfile`，不得改写 lockfile。
3. 在可访问 registry 的环境运行 `corepack pnpm audit --audit-level=moderate`，结果必须为
   零个 moderate/high/critical 漏洞；registry 故障应使门禁失败，而不是跳过。
4. 运行 `corepack pnpm test -- --maxWorkers=2` 和 `corepack pnpm build`。
5. 检查 CI：`gui-audit` 应在固定 pnpm/frozen install 后执行，release job 必须依赖它。

验收过程中不应出现 `package-lock.json`，也不应通过 npm、npx 或 yarn 安装 GUI 依赖。
