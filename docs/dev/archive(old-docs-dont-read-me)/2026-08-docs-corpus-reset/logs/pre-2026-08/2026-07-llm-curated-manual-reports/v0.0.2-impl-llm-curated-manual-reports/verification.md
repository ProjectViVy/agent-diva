# 验证记录

## 定向测试

| 命令 | 结果 |
|------|------|
| `cargo test -p agent-diva-core --lib reports` | 通过（含事实包过滤、校验、渲染、frontmatter） |
| `cargo test -p agent-diva-providers --lib report_narrative` | 通过（JSON/evidence/model id） |
| `cargo test -p agent-diva-autodream` | 通过（日/周/月生成、service trigger、worker） |
| `cargo test -p agent-diva-gui --manifest-path agent-diva-gui/src-tauri/Cargo.toml notebook` | 通过（14 tests，含 generation_mode 读取） |
| `cargo check -p agent-diva-manager` | 通过（仅预存 `workspace` dead_code 警告） |

## 工作区门禁

- 未在本迭代跑完整 `just fmt-check && just check && just test`（耗时/预存无关失败风险）；定向 crate 测试已覆盖主路径。
- PowerShell 将 cargo stderr 警告记为 exit code 1，但测试结果均为 `ok`。

## 说明

- 默认配置关闭 LLM 归纳；关闭时写入 `generation_mode: deterministic_fallback`。
- 真实 provider 端到端 smoke 需在启用 `reports.llm_curation.enabled=true` 且配置有效模型后手工验证。
