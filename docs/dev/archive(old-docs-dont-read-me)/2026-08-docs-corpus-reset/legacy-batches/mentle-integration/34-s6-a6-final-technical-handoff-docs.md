# Sprint 6 A6: Final Technical Handoff Docs (Mentle Integration)

## 1. 总体说明 (Mentle Integration Overview)

Mentle 与 Agent-Diva 的集成在 Sprint 1 至 Sprint 6 期间完成。本次集成作为 RC (Release Candidate) 基线，成功在 Agent-Diva 内部嵌入了 Mentle 的 Palace memory (L2)，同时保持了原有 L0/L1 Markdown 内存的连贯性。Mentle 以可选能力形式加载，并拥有严格的运行时组装控制及降级机制，不会默认侵入标准无 Mentle 的 Agent-Diva 实例。

Sprint 6 A6 作为收口环节，负责梳理交付基线，产出面向开发者与操作者的总体行为预期、验证策略与遗留边界。

## 2. Package Source Policy

依据基线强制要求，Mentle 集成必须通过 crates.io 发布的正式 Cargo 依赖包进行组装。

- **Crate**: `memtle`
- **Source**: `crates.io`
- **Version**: 精确锁定为 `0.1.2`
- **Features**: `default-features = false`

**禁止行为**：禁止在工作区中通过 `path = "../mentle"`、`git` 或 `[patch.crates-io]` 进行本地依赖覆盖。此类修改仅可用于本地实验，必须在推送到默认分支和构建 Release 前清理。升级版本必须进行单独评审。

## 3. Feature Gate 说明

Mentle 功能被严格隔离在 Cargo 的可选 Feature 下：

- 默认情况下的 Agent-Diva 构建（即没有指定 `mentle` feature）将不包含任何 Mentle 依赖和逻辑，保持零侵入。
- 启动 Mentle 集成：必须在编译和运行时显式指定 `mentle` feature（如 `cargo check -p agent-diva-agent --features mentle`）。
- **Runtime Assembly**：Mentle 运行时仅在成功获得 `memtle_status` 锚点工具后，才触发 Mentle 相关 prompt 的激活，不会凭空向模型广告虚假的记忆能力。

## 4. 降级行为说明 (Downgrade Behaviors)

Agent-Diva 制定了完善的运行时容错降级合约，确保由于 Mentle 引起的局部失败不会影响主核心回路：

- **Startup Failure**：如果 Mentle 初始化失败（例如因目录权限或配置失败），Agent-Diva 会正常启动并自动降级回 Markdown 内存模式，不会提供 `memtle_*` 工具，并在 prompt 中禁用对应提示。
- **Recall Failure**：在 AgentLoop 中的 Mentle recall（记忆召回）若发生查询失败，该过程为非致命错误，不会中断当前的请求和模型响应。
- **Write Failure**：当 `memtle_diary_write` 执行失败时，文件系统的历史记录 (Markdown history) 将依然保持权威性，防止数据丢失。
- **Invalid Definition Handling**：若 Mentle toolkit 提供或注入了无效的动态工具定义，将被单独跳过并记录警告，不会导致整个运行时崩溃。

## 5. 已知限制 (Known Limitations)

当前 RC 基线中存在以下已知限制和环境要求：

- **编译前置条件（Windows）**：`mentle` 特性在 Windows 系统下要求 Rust `>= 1.88.0` 并依赖本地 Native C/C++ 工具链。若出现缺失 `clang-cl.exe` 等编译异常，需要手动向 PATH 添加对应工具链（如 `$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH`）。
- **`just test` 失败问题**：在执行全工作区的 `just test` 时，因 `agent_diva_providers::ollama` 导出问题可能会报告 provider 导出测试失败，该失败项独立于 Mentle 集成之外，目前接受此异常继续推进 RC。
- **`with_toolset()` 行为**：使用该注册机制时，目前不自动安装 Mentle hybrid memory provider，保持外部注册表驱动的设计边界。

## 6. 不支持项 (Unsupported Items in RC Scope)

为确保发布的稳定性，以下功能由于超出 RC 范围在 Sprint 6 内明确不予支持（将在后续跟进）：

- GUI 对 Mentle 各级模式（如开启/关闭、仅读取、仅特定工具等）的视觉控制面板。
- Mentle 工具集的“热重载” (Runtime hot-refresh)。一旦运行前组装完毕，动态工具定义在生命周期内不再刷新。
- 子 Agent (Subagents) 默认情况下的 Mentle 长效内存与能力继承（为保证状态隔离，默认下 subagent 会禁用 Mentle）。
- 任何由于用户私自调整依赖路径到本地 `mentle/` 而引起的构建版本异常。

## 7. 后续 Sprint 7 候选增强项

以下增强项被归类在 Sprint 7 (S7-A1 及以后)，待 RC 获得最终验收后启动：

- **Mentle Tool Mode Selection**：支持基于配置设定具体的操作模式，包括 `off`, `read_only`, `full`, `custom` 等，以便精细化过滤动态工具注册。
- **GUI Controls**：在 `agent-diva-gui` 增加通用设置项，支持动态开关 Mentle 及其工具组合，支持应用后保存持久化配置并触发运行时环境的安全重建。
- **Per-tool Activation**：使用动态 Checklist 列出 `memtle_*` 可用工具列表，允许用户勾选。

## 8. Operator / Developer Usage Notes

### 8.1 开发者验证指令

针对 Mentle 集成开发，执行验收或常规验证请使用：

```powershell
# Default Lane（保证隔离性，无 Mentle）
just sprint5-default-check

# Mentle Feature Lane（验证 Mentle 组装与能力）
just mentle-check

# 验证包来源规范约束
cargo run --manifest-path scripts/ci/verify_mentle_package_policy.py
```

如果在 Windows 报编译器错误，请在当前 Terminal 执行类似 `$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH` 以引入对应编译工具后再运行。

### 8.2 运营与运维指南

- Mentle 提供的高级记忆工具通过 `memtle_*` 开头的动态定义执行。如需屏蔽，需要通过启动参数或避免开启对应 Cargo Feature。
- 不用担心数据库文件的损坏阻断当前核心流程，系统已经建立容错机制转到常规 Markdown 文件作为回退备用机制。
- Review 与验收需紧扣本 RC 边界（[Sprint 6 A1 Baseline](./29-s6-a1-rc-scope-baseline.md)），拒绝任何新增的“热更新”和 GUI 视图修改代码在此阶段直接合入。

---
*Created by Writer under Gemini 3.1 Pro route. Verified against current RC baseline and S5 failure semantics docs.*
