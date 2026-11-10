# 验证

| 检查 | 结果 |
| --- | --- |
| README 本地链接、章节锚点和 npm 脚本名 | 两份均通过，各 8 个本地链接 |
| 项目文档站和旧版插画地址 | 均返回 HTTP 200 |
| `npm ci` | 通过。当前 Node 24.14.1 低于两个依赖声明的 24.15.0，另有 5 项中等严重性审计报告 |
| `npm run build` | 通过，含 Vue 类型检查 |
| `npm test` | 9 个文件，44 项通过 |
| `npm run test:artifact` | 5 项通过 |
| `cargo test --manifest-path bridge-tests/Cargo.toml --locked` | 使用工作区内的临时目录重跑后 10 项通过，5 项按设计忽略。默认 Windows 临时目录首次运行返回访问拒绝 |
| Vite 合成 IPC 模式 | 主页和转换后的 Vue 入口均返回 HTTP 200 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 上游 `bridge.rs` 中一处多行调用格式差异，见 [待办](../../../../TODOLIST.md) |
| `git diff --check` 和产品文件对比 | 文档补丁无空白错误，产品源码和工作流与 `8468ca94` 一致 |

当前 checkout 缺少匹配的 Windows x64 核心产物，原生桥接、桌面首启和 NSIS 安装待用匹配产物验证。新版采用根目录 npm 脚本和独立 Cargo manifest，以上命令对应当前 CI。
