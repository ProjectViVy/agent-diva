## Acceptance

## Product Acceptance Steps

1. 打开 `docs/app-building/wbs-headless-cli-package.md`，确认其只覆盖 `CA-DIST-CLI-PACKAGE`，没有混入 GUI 安装器内容。
2. 确认文档包含 CA/WP 责任边界、输入/输出、完成定义（DoD）与分阶段里程碑。
3. 确认文档包含明确的包命名规范、目录结构、`bundle-manifest.txt` 字段、`bin/` 启动路径。
4. 确认文档提供 PowerShell/Bash 打包片段、CI 工件上传、Release 失败门禁、QA 映射。
5. 打开 `docs/app-building/README.md`，确认 Headless 索引与阶段建议已链接到该专项文档。
6. 打开 `docs/app-building/headless-bundle-quickstart.md`，确认其最小运行路径与 `bin/` 包结构一致。
7. 打开 `docs/app-building/wbs-distribution-and-installers.md`，确认 `CA-DIST-CLI-PACKAGE` 总览页已指向 companion 文档，而不是保留孤立描述。

## Acceptance Result

- 当前版本满足上述验收项，可作为 `CA-DIST-CLI-PACKAGE` 的阶段性主文档，用于后续实现拆解与任务下发。
