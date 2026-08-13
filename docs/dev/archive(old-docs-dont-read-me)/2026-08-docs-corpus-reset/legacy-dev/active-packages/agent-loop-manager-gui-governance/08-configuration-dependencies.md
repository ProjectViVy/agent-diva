# 配置与依赖管理

## 1. 本计划默认无新配置

Phase 0–6 不新增用户配置、环境变量、Cargo crate 或 npm package。治理只重组当前实现。

## 2. 配置所有权

| 配置 | 权威层 | GUI 行为 |
| --- | --- | --- |
| provider/model | core/provider + Manager | 读取/提交 intent，不保存第二份 |
| tool/sandbox | core/sandbox + Manager | 展示 projection |
| gateway port | Manager/Tauri Host | Host 发现，组件不可硬编码 |
| GUI prefs/theme | Tauri LOCAL | 不进入 Manager domain |
| pet assets/voice | Tauri LOCAL | 路径受 config dir 约束 |
| session/plan | domain store | 禁止作为 GUI config 保存 |

当前 Tauri 通过 [AgentState](../../../agent-diva-gui/src-tauri/src/app_state.rs:13) 读取/更新 Manager 地址，这一能力在 API 拆分期间保留。

## 3. 可选代码生成依赖

`specta`、`ts-rs` 或 JSON Schema generator 只能在 Phase 5 单独 ADR 后引入。评估维度：

- Rust 1.80/MSRV；
- enum/tag/optional 字段表达；
- Tauri 与纯 HTTP DTO 支持；
- 生成物是否可复现；
- 是否迫使 domain 类型依赖 presentation；
- CI 和 Windows 构建成本。

在决策前使用 fixture contract tests，不新增依赖。

## 4. 安全

- API key/token 不进入生成 fixture、GUI store、日志或 error；
- LOCAL path 必须沿用 config dir/workspace 边界；
- 前端直连 Manager 若未来实施，必须先解决 loopback bind、CORS/CSP、认证与恶意网页调用风险；
- 不因拆分 `commands.rs` 放宽 Tauri command allowlist。
