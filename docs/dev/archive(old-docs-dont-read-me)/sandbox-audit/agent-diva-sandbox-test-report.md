# agent-diva-sandbox 测试报告

> 测试日期: 2026-06-01
> 环境: Windows 10, Rust 1.80.0+, agent-diva-with-sandbox 分支

## 执行摘要

| 测试目标 | 结果 | 通过/总数 |
|----------|------|-----------|
| agent-diva-sandbox 单元测试 | ✅ 全部通过 | 90/90 |
| agent-diva-tools 测试 | ⚠️ 2 个失败 | 75/77 |
| 全项目编译 (cargo check) | ❌ CLI 编译失败 | - |

## 详细结果

### 1. agent-diva-sandbox crate (90/90 通过)

```
test result: ok. 90 passed; 0 failed; 0 ignored
```

测试覆盖模块：
- approval: 6 tests (缓存 CRUD、denied 检查、key 匹配)
- decision: 8 tests (解析、聚合、序列化、排序)
- exec_policy: 10 tests (规则匹配、审批需求、禁止前缀)
- filesystem: 5 tests (访问模式、路径保护、路径遍历防护)
- guardian: 12 tests (熔断器、自动审批、危险检测)
- manager: 6 tests (配置、禁用、直接执行)
- orchestrator: 17 tests (编排器创建、审批检查、沙箱尝试、重试逻辑)
- platform/windows: 6 tests (Token 创建、执行器)
- rules: 8 tests (规则匹配、TOM 序列化)
- 其他: 12 tests

### 2. agent-diva-tools (75/77 通过，2 失败)

#### 失败 #1: test_exec_does_not_fallback_to_direct_execution_after_sandbox_failure
```
assertion failed: !result.contains("hello")
```
**说明**: 此测试验证"沙箱失败后不应回退到直接执行"。但测试失败了——沙箱失败后**确实回退到了直接执行**（输出中包含 "hello"）。
**严重性**: 🔴 关键 — 验证了安全审查中发现的 fallback 绕过路径。

#### 失败 #2: test_read_file_with_offset_limit
```
assertion failed: result.contains("[Lines 2-3 of 5]")
```
**说明**: filesystem 工具的 offset/limit 格式化输出不匹配预期。
**严重性**: 🟡 中等 — 可能是 filesystem.rs 的沙箱分支改动导致。

### 3. 全项目编译

```
error[E0063]: missing field `approval_store` in initializer of `ToolConfig`
  --> agent-diva-cli\src\chat_commands.rs:69:23
```
**说明**: CLI crate 中 ToolConfig 初始化缺少 `approval_store` 字段。
**严重性**: 🟡 中等 — 预期内（迁移计划已标注），需要适配迁入。

## 测试覆盖盲区

| 盲区 | 说明 |
|------|------|
| 平台沙箱实际可用性 | Windows RestrictedToken、Linux Landlock/Seccomp、macOS Seatbelt 仅在单元测试中 mock，未在真实环境中验证 |
| 审批流端到端 | 未测试完整审批→执行→重试的真实流程 |
| 并发安全 | ApprovalStore 的多线程并发访问未充分测试 |
| 网络访问控制 | 无 network access 的实际阻断测试 |
| protected paths 真实路径 | 仅单元测试路径匹配逻辑，未测试实际文件系统操作 |
| 安全边界渗透 | 未运行任何恶意命令渗透测试 |

## 建议

1. **修复 P0**: test_exec_does_not_fallback 失败确认了沙箱绕过路径存在
2. **补充测试**: 添加针对 shell metacharacter 绕过、路径规范化绕过、环境变量绕过的安全测试
3. **平台验证**: 在真实 Windows/Linux 环境中验证平台沙箱的实际可用性
4. **修复 CLI**: 完成 ToolConfig 的 approval_store 字段集成
