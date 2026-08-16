# 验证记录

| 门 | 命令 | 结果 |
| --- | --- | --- |
| 现场复现 | `GET http://127.0.0.1:3000/api/persona/status` | 旧信封 `{ status: { status: "uninitialized", files: ... } }` |
| Tauri 解码 | `cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml --lib laputa_payload_tests` | PASS（4/4） |
| Manager Persona API | `cargo test -p agent-diva-manager persona_api_initializes_reads_and_enforces_cas` | PASS |
| 格式 | `just fmt-check` | PASS |
| Clippy | `cargo clippy -p agent-diva-manager --all-targets -- -D warnings` | PASS |
| Clippy GUI | `cargo clippy --manifest-path agent-diva-gui/src-tauri/Cargo.toml --all-targets -- -D warnings` | PASS |
| 全量 `just test` | 未跑 | 本片只改 Persona HTTP 信封与 Tauri 解码；行为由上述聚焦测试覆盖 |

## 观察点

- 新信封：`status == "ok"` 且 `persona.status` 为 `uninitialized` / `ready`。
- 旧信封：HTTP 200 + 对象 `status` 不得再变成 `unknown Laputa API error`。
- 422 `{ error: { message } }` 必须把 `message` 交给 GUI。
