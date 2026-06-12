## Summary

- **Iteration**: `2026-03-headless-cli-package`
- **Version**: `v0.0.1-tech-enhanced-wbs`
- **Scope**: 本次交付只覆盖 `CA-DIST-CLI-PACKAGE` 的文档化实施方案，不改动核心业务代码与运行时行为。

### What Changed

- 新增 `docs/app-building/wbs-headless-cli-package.md`
  - 为 `CA-DIST-CLI-PACKAGE` 单独建立技术增强版 WBS 文档。
  - 固化了控制账户边界、负责人、输入/输出、完成定义（DoD）。
  - 明确了跨平台包命名、目录结构、`bundle-manifest.txt` 契约、Phase 1/2/3 里程碑。
  - 补入了 PowerShell/Bash 打包脚本片段、CI 工件上传、Release 门禁、QA 映射。

- 更新 `docs/app-building/README.md`
  - 把 `wbs-headless-cli-package.md` 纳入文档索引。
  - 在 Headless 文档映射与阶段建议中明确该文档是 `CA-DIST-CLI-PACKAGE` 的主执行文档。
  - 将 `headless-bundle-quickstart.md` 的定位更新为 Phase 1 占位 README。

- 更新 `docs/app-building/wbs-distribution-and-installers.md`
  - 在 `CA-DIST-CLI-PACKAGE` 小节增加 companion 文档指引。
  - 将最小启动路径从包根目录统一为 `bin/agent-diva(.exe)`。
  - 将示例配置模板路径收敛为 `config/config.example.json` 和 `config/env.example`。

- 更新 `docs/app-building/headless-bundle-quickstart.md`
  - 将随包结构与最小启动路径同步到新的 `bin/` 布局。
  - 增加对 `services/README.md` 与专项 WBS 的引用，避免后续实现继续沿用旧布局。

### Why It Matters

- 之前 `CA-DIST-CLI-PACKAGE` 在总览与总 WBS 中只有最小描述，缺少可直接实现的包契约和脚本级实践。
- 本次补齐后，后续 Agent 或工程师可以直接按文档实现：
  - 本地打包脚本；
  - CI Headless 包工件产出；
  - Release 资产命名与门禁；
  - Phase 1 到 Phase 3 的分阶段推进。
- 同时保持最小侵入：没有扩散到 `agent-diva-core`、`agent-diva-agent`、`agent-diva-providers`、`agent-diva-channels`、`agent-diva-tools`。
