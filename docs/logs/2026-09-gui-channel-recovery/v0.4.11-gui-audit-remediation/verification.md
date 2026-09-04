# GUI dependency audit remediation verification

日期：2026-09-04

## 已通过

- `corepack pnpm install --frozen-lockfile`：通过。
- 修复时执行 `corepack pnpm audit --audit-level=moderate`：返回零漏洞。
- `python scripts/ci/check_gui_dependency_policy.py`：通过；pnpm-only、单 lockfile、
  CI audit job 和 release dependency 均被静态门禁覆盖。
- 离线 frozen install（忽略 lifecycle scripts）：通过，证明提交的 lockfile 可复现解析。
- GUI 全量测试：77 个文件、541 个测试通过；`corepack pnpm build` 通过。
- CI YAML 解析、Windows PowerShell packaging AST、`just` dry-run 和
  `git diff --check`：通过。
- active scripts/CI/文档扫描：没有 npm、npx 或 yarn 调用。

## 如实保留的环境结果

后续独立复核尝试再次访问 registry 时，audit 请求长时间没有返回，因此没有把这次请求
记作第二次通过。依赖图已经以精确安全版本提交；未来每个 CI run 仍必须通过独立、带超时且
fail-closed 的 `gui-audit` job。网络服务不可用不会被降级为成功。

后续频道设置变更增加了 GUI 用例，最终整体验证记录在相邻的
`v0.4.12-channel-settings-recovery/verification.md`，不以本页较早的 541 个测试覆盖该结果。
