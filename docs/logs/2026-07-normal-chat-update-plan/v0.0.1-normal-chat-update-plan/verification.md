# 普通聊天 update_plan TODO 清单 — 验证记录

## 验证策略

- 针对改动 crate 跑单元/集成测试。
- 对改动文件跑 `cargo fmt --check`。
- 对改动 crate 跑 `cargo clippy -- -D warnings`（并记录预存阻塞项）。
- 运行 CLI `--help` 作为最小 smoke test。
- 由于缺少 mock provider，使用 `agent-diva-cli/tests/update_plan_e2e.rs` 作为 CLI 路径的功能 smoke test。

## 执行命令与结果

### 1. 针对性测试

```bash
cargo test -p agent-diva-core -p agent-diva-tools -p agent-diva-agent -p agent-diva-cli update_plan
```

**结果**：PASS
- `agent-diva-core`: 10 个 `update_plan` 相关测试全部通过（含序列化、非法状态、未知字段、常量）。
- `agent-diva-tools`: 8 个 `update_plan` 工具测试全部通过（含合法、空 plan、非法 status、超长、超数）。
- `agent-diva-agent`: 5 个测试全部通过（含普通聊天注册、Plan 模式未注册、Execution 模式未注册、handler 发事件、系统提示词包含 `update_plan`）。
- `agent-diva-cli`: 1 个集成测试 `update_plan_end_to_end_client_sse` 通过。

证据：`.sisyphus/evidence/f1-targeted-update-plan-tests.log`

### 2. 格式化检查

```bash
cargo fmt --check
```

**结果**：PASS

证据：`.sisyphus/evidence/blockers-fmt-check-after-apply.log`

### 3. Clippy 检查

```bash
cargo clippy --workspace -- -D warnings
```

**结果**：PASS（唯一剩余提示是 `imap-proto v0.10.2` 的 future-incompat 警告，与本项目代码无关）。

证据：`.sisyphus/evidence/blockers-final-clippy.log`

### 4. CLI Smoke Test

```bash
cargo run -p agent-diva-cli -- --help
cargo run -p agent-diva-cli -- agent --help
cargo run -p agent-diva-cli -- chat --help
cargo run -p agent-diva-cli -- tui --help
```

**结果**：PASS — CLI 可编译启动，子命令帮助正常输出。

证据：`.sisyphus/evidence/task-11-cli-help.log` 等

### 5. 完整 Workspace 检查

```bash
cargo test --workspace update_plan
```

**结果**：PASS — 阻塞已清理，完整 workspace 编译并运行成功。

证据：`.sisyphus/evidence/blockers-final-workspace-test.log`

## 已清理阻塞项

- `agent-diva-core/src/planning/report.rs`：合并冗余 `if`/`else if` 分支，消除 `clippy::if_same_then_else`。
- `agent-diva-providers/tests/{provider_retry,ollama_tools,ollama_streaming}.rs`：补充 `ToolChoiceMode::Unspecified` 参数。
- `agent-diva-neuron/tests/neuron_smoke.rs`：mock provider 的 `chat` 方法补充 `ToolChoiceMode` 参数。
- `agent-diva-e2e/src/assertions.rs`：mock provider 与调用点补充 `ToolChoiceMode` 参数。
- `agent-diva-e2e/src/collector.rs`：补充 `AgentEvent::ChatPlanUpdate` match 分支。
- `agent-diva-migration/src/config_migration.rs`：补充 `reports` 与 `response_protocol` 字段。
- `agent-diva-providers/src/openai_compatible.rs`：为 `new_with_config` 添加 `#[allow(clippy::too_many_arguments)]`。
- `agent-diva-manager/src/manager.rs`：为未使用的 `workspace` 字段添加 `#[allow(dead_code)]`。

## 验证结论

- `update_plan` 功能路径在 workspace 范围内测试通过。
- `cargo fmt --check` 通过。
- `cargo clippy --workspace -D warnings` 通过。
- 剩余唯一非阻塞项是 `imap-proto` 的 future-incompat 警告，不影响当前版本。
