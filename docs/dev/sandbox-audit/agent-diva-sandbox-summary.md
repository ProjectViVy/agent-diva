# agent-diva-sandbox 综合审查总结

> 审查日期: 2026-06-01
> 审查团队: 3 个子代理 (deepseek-v4-pro) + 主代理协调
> 审查范围: agent-diva-with-sandbox 分支全部变更 (~5300 行/40 文件)

---

## 一、最终结论

### 判定: ⚠️ MIGRATE WITH FIXES（修复后迁移）

agent-diva-sandbox 的核心设计方向正确，但存在 **2 个关键安全缺陷**和若干架构问题，**不能直接合入主线**。建议在修复 P0/P1 问题后按推荐顺序分批迁移。

---

## 二、当前状态

| 项目 | 状态 |
|------|------|
| 分支 | `agent-diva-with-sandbox`（本地，未推送） |
| HEAD | `50f58c2`（与 main 相同，所有改动在工作区） |
| 工作区 | 34 文件修改 + 多个新文件未跟踪 |
| 测试 | sandbox crate 90/90 通过；tools 75/77 (2 失败) |
| 编译 | sandbox crate ✅；全项目 ❌（CLI 缺少字段） |

---

## 三、审查发现总结

### 架构评分: 2.8/5.0

| 维度 | 评分 | 说明 |
|------|------|------|
| 模块职责清晰度 | 3/5 | 基础模块清晰，Orchestrator 职责过重 |
| 数据流合理性 | 3/5 | 审批检查出现三次，缓存访问破坏封装 |
| 状态机正确性 | 3/5 | 设计完整但有逃逸路径 |
| 耦合度控制 | 3/5 | 无循环依赖，Orchestrator 依赖 7/10 模块 |
| 错误处理 | 2/5 | 非零退出码=Ok，ApprovalRequired 双重语义 |
| 公共 API | 2/5 | 重导出过多，类型命名不一致 |

### 安全问题: 2 CRITICAL + 4 HIGH + 3 MEDIUM

| 级别 | 问题 | 验证 |
|------|------|------|
| 🔴 CRITICAL | Shell 命令重新拼接导致注入：`shell_words::split` 后用 `join(" ")` 重新拼接传给 shell，所有 metacharacter 可绕过 | 静态分析 |
| 🔴 CRITICAL | Windows RestrictedToken 形同虚设：硬编码不可用 → 自动触发升级 → 无沙箱执行 | **测试确认**（test_exec_does_not_fallback 失败） |
| 🟠 HIGH | macOS sandbox-exec 不可用时静默 fail-open | 静态分析 |
| 🟠 HIGH | Guardian 默认 auto_approve_known_safe=true + auto_learning=true | 静态分析 |
| 🟠 HIGH | BANNED_PREFIX 可被路径别名绕过 | 静态分析 |
| 🟠 HIGH | Windows 平台完全无网络访问控制 | 静态分析 |
| 🟡 MEDIUM | protected paths 缺少 .env.*、.tfvars、凭证文件 | 静态分析 |
| 🟡 MEDIUM | cargo/rustc/rustup 被错误标记为只读 | 静态分析 |
| 🟡 MEDIUM | 路径 starts_with 检查无规范化 | 静态分析 |

### 测试覆盖

- ✅ sandbox crate: 90/90 单元测试通过
- ❌ `test_exec_does_not_fallback_to_direct_execution_after_sandbox_failure` — **确认了 CRITICAL #2**
- ❌ `test_read_file_with_offset_limit` — filesystem 集成问题
- ❌ 全项目 cargo check 失败 — CLI approval_store 未适配

---

## 四、12 个审查问题解答

| # | 问题 | 答案 |
|---|------|------|
| 1 | 核心设计是否值得迁入？ | ✅ 是。SandboxPolicy/ToolOrchestrator/平台抽象设计方向正确 |
| 2 | 默认策略是否足够保守？ | ⚠️ 部分。SandboxMode 默认 WorkspaceWrite 可接受，但 Guardian 默认太宽松 |
| 3 | 是否存在绕过路径？ | 🔴 是。2 个 CRITICAL 绕过已验证 |
| 4 | OnFailure 是否过度提权？ | ⚠️ 是。Windows 上 sandbox 不可用 → 自动升级 → 无沙箱执行 |
| 5 | Guardian 自动学习是否应默认关闭？ | ✅ 是。enable_auto_learning 和 auto_approve_known_safe 必须默认 false |
| 6 | Windows RestrictedToken 是否有效？ | 🔴 否。当前实现不可用，会自动降级 |
| 7 | Linux/macOS 不可用时 fallback？ | 🔴 fail-open。macOS 静默降级到无沙箱 |
| 8 | command prefix 是否可绕过？ | 🟠 是。路径别名 (/usr/bin/python3 vs python3) 可绕过 |
| 9 | protected paths 是否完整？ | 🟡 否。缺少 .env.*、.npmrc、.tfvars 等 |
| 10 | 与主线 SecurityPolicy 如何合并？ | 🟡 类型重复。sandbox 的 SandboxPolicy 和主线的 SecurityPolicy 需统一 |
| 11 | 最小迁移切片是否只做 ExecTool？ | ✅ 是。PR#1 只迁 sandbox crate + 配置，零行为影响 |
| 12 | 是否需要先实现 Phase B trace log？ | ⚠️ 建议先迁 sandbox 引擎（PR#1），trace log 可与 PR#2 并行 |

---

## 五、迁移计划

### PR #1: 沙箱引擎 + 配置层（推荐首批，0.5 天）
```
agent-diva-sandbox/          # 全新 crate
agent-diva-core/src/config/  # 追加 4 个类型
Cargo.toml                   # workspace member
```
- 零行为影响，安全合入
- 验证: `cargo build -p agent-diva-sandbox && cargo test -p agent-diva-sandbox`

### PR #2: Shell 工具 + Agent 循环（核心 PR，1-2 天）
```
agent-diva-tools/src/shell.rs        # 341 行改造
agent-diva-agent/src/agent_loop.rs   # ToolConfig 扩展
agent-diva-agent/src/subagent.rs     # 参数签名变更
agent-diva-cli/                      # 初始化适配
```
- 需先修复 P0 问题
- 验证: 全项目编译 + 现有测试 + shell 手动测试

### PR #3: Manager 运行时集成（1 天）
```
agent-diva-manager/  # 7 文件变更
```
- approval_store 穿线

### PR #4: GUI 设置页面（可选，0.5 天）
```
agent-diva-gui/  # Vue/Tauri 变更
```

---

## 六、整改优先级

```
P0 (合入前必须修复):
  1. 🔴 修复 shell 命令重新拼接导致的注入路径
  2. 🔴 修复 Windows RestrictedToken fallback 逻辑（确保不静默降级）
  3. 🔴 修复 test_exec_does_not_fallback 测试

P1 (第一批合入前):
  4. 🟠 macOS fail-open 改为 fail-closed
  5. 🟠 Guardian 默认 auto_approve_known_safe=false, auto_learning=false
  6. 🟠 BANNED_PREFIX 增加路径别名匹配

P2 (第二批):
  7. 🟡 扩展 protected paths 覆盖
  8. 🟡 修复路径 starts_with 匹配
  9. 🟡 统一审批缓存（消除双重缓存）
  10. 🟡 非零退出码改为 Err

P3 (后续优化):
  11. 🟢 Orchestrator 解耦
  12. 🟢 API 清理 + feature gate
  13. 🟢 与主线 SecurityPolicy 统一
```

---

## 七、审查文档索引

| 文档 | 路径 | 内容 |
|------|------|------|
| 审查计划 | docs/dev/sandbox-audit/agent-diva-sandbox-audit-plan.md | 基线信息 + 审查范围 |
| 架构审查 | docs/dev/sandbox-audit/agent-diva-sandbox-code-review.md | 637 行，模块分析 + 评分 |
| 迁移计划 | docs/dev/sandbox-audit/agent-diva-sandbox-migration-plan.md | 505 行，diff 分析 + 切片建议 |
| 测试报告 | docs/dev/sandbox-audit/agent-diva-sandbox-test-report.md | 测试结果 + 盲区 |
| 综合总结 | docs/dev/sandbox-audit/agent-diva-sandbox-summary.md | 本文档 |
