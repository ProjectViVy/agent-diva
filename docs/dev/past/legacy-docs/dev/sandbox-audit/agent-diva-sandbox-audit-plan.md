# agent-diva-sandbox 审查计划

## 基线信息

- **分支**: `agent-diva-with-sandbox` (本地未推送)
- **HEAD**: `50f58c2` (与 main 相同, 所有改动在工作区)
- **工作区状态**: 34 个文件修改, 1402 行新增, 59 行删除, 多个未跟踪文件
- **sandbox crate**: `agent-diva-sandbox/` (新 crate, 未跟踪)
- **代码量**: 17 个 Rust 源文件 + README + Cargo.toml
- **Claude Code**: v2.1.159 已安装可用

## 审查轮次

### Round 1: 架构审查 (子代理 A)
审查模块职责、数据流、状态机
文件: policy.rs, orchestrator.rs, manager.rs, approval.rs, decision.rs, guardian.rs, lib.rs, error.rs, filesystem.rs

### Round 2: 安全审查 (子代理 B)
审查命令解析安全性、绕过路径、fallback 行为
文件: exec_policy.rs, rules.rs, platform/windows.rs, platform/linux.rs, platform/macos.rs, policy.rs

### Round 3: 集成审查 (子代理 C)
与主线对照, 迁移 diff 分析
文件: agent-diva-tools/src/shell.rs (两边), agent-diva-agent/src/tool_assembly.rs, agent-diva-core/src/security/*, config/schema.rs

### Round 4: 测试执行 (主代理)
运行已有测试、安全测试用例设计

## 审查问题清单
1. agent-diva-sandbox 的核心设计是否值得迁入主线？
2. 默认策略是否足够保守？
3. 是否存在 approval bypass 或 sandbox bypass 的路径？
4. OnFailure 是否会在失败后过度提权？
5. Guardian 自动学习是否应该默认关闭？
6. Windows Restricted Token 实现是否真的有效？
7. Linux/macOS 不可用时 fallback 是 fail-closed 还是 fail-open？
8. command prefix matching 是否能被 shell metacharacter 绕过？
9. protected paths 是否覆盖 .git/.env/密钥/agent 私有目录？
10. 当前主线已有 SecurityPolicy 与 sandbox 分支的 policy 如何合并？
11. 最小迁移切片是否应只做 ExecTool？
12. 是否需要先实现 Phase B trace log，再迁 sandbox？

## 产出要求
- [ ] 架构审查报告: 模块职责、数据流、状态机、边界
- [ ] 安全审查报告: 审批绕过、命令匹配、路径保护、fallback、平台差异
- [ ] 测试报告: 已跑命令、结果、失败项、未覆盖项
- [ ] 迁移计划: 推荐顺序、第一 PR 切片、不建议迁移的部分、与 Phase B 衔接点
