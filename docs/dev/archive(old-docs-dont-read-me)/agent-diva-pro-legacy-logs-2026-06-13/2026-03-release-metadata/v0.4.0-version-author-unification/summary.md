# v0.4.0 Version And Author Unification

## 变更摘要

- 将工作区主项目 crate 的版本统一为 `0.4.0`。
- 将项目作者标识统一为 `mastwet (projectViVY Team, undefine foundation)`。
- 同步更新了 CLI、GUI、打包脚本、打包文档中的版本展示与作者展示。
- 同步更新了 `.workspace/nano-workspace/agent-diva-nano` 的版本与作者信息，保持项目自有扩展包一致。

## 影响范围

- Rust workspace manifests 与内部 path 依赖版本约束。
- CLI 版本输出与内嵌元数据。
- GUI `package.json`、Tauri Rust 包元数据、About 页面展示。
- Linux/macOS/zip 打包脚本和发布文档。
