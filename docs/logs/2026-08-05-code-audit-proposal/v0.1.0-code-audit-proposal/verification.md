# agent-diva 代码审查预案验证方法与测试策略

## 1. 验证目标

本文档定义后续执行代码清理提案时的验证方法，确保在移除旧版兼容层与死代码的过程中：
1. 不破坏现有的核心业务逻辑与 API 契约；
2. 不会重新引入已被淘汰的依赖或架构隐患（Clean-break 保障）；
3. 确保全部自动化测试与代码检查（Formatting, Clippy, Unit tests）全量通过。

---

## 2. 自动化验证门禁命令

在实施任一清理提案（Proposal 01 ~ 05）后，均需按顺序执行以下命令进行回归验证：

```bash
# 1. 格式化检查
just fmt-check

# 2. Workspace Clippy 检查（严禁死代码或未用变量告警）
just check

# 3. 记忆系统 clean-break 规则检查（验证旧依赖无回流）
just laputa-clean-break-check

# 4. 全量 Workspace 单元与集成测试
just test

# 5. （针对 GUI 相关的提案 03）GUI 代码检查与单元测试
cargo check -p agent-diva-gui --all-targets
```

---

## 3. 针对各提案的专项验证方案

### 3.1 Proposal 01: Legacy Memory Boundary 清理验证
- **断言方法**：搜索 `MemoryAuthorityMode::Legacy` 与 `MemoryAuthorityMode::Shadow` 引用，确保在编译期与运行时均无残余。
- **回归测试**：运行 `cargo test -p agent-diva-agent memory_boundary` 验证 `LaputaRecallService` 与 `DegradedMemoryProvider` 正常开箱即用。

### 3.2 Proposal 02: Stub Planning Tools 清理验证
- **断言方法**：检查 `PlanApproveTool` 移除后 `tool_assembly.rs` 的构建情况，确保 AgentLoop 在 Plan / Ask / Execute 各模式下的 Tool List 严格正确。
- **回归测试**：运行 `cargo test -p agent-diva-agent planning`。

### 3.3 Proposal 03: GUI 双通道 SSE 整合验证
- **断言方法**：前端 `App.vue` 仅保留 `approval-event` SSE 监听器，UI 仅维护 `unifiedApprovals`。
- **自动化测试**：运行 Tauri 前端 Vite 构建与 Vitest 测试：
  ```bash
  cd agent-diva-gui && npm run test:unit && npm run build
  ```

### 3.4 Proposal 04: Dead Code 标记清理验证
- **断言方法**：移除 `#[allow(dead_code)]` 后，运行 `cargo clippy --all -- -D warnings` 必须 0 警告通过。若有遗漏未调用的代码，Clippy 将准确捕获。

### 3.5 Proposal 05: AutoDream 与 Migration 逻辑收口验证
- **断言方法**：运行 `cargo test -p agent-diva-autodream` 与 `cargo test -p agent-diva-migration`，确认旧数据格式转入不确定性捕获（fail-closed）。
