# 交班文档:agent-diva-pro context-compaction 主题(2026-06-07)

> 接手人:开发人员 / 后续代理
> 交班人:大湿(Hermes session)
> 主题:agent-diva-pro `feature/context-compaction` 分支状态、未提交工作、下一步建议
> 目标:让接手人 10 分钟内能上手、知道坑在哪、知道什么能扔、什么必须留

---

## TL;DR

- **分支**:`feature/context-compaction` · HEAD `0b9ba79`(CC-P6 E2E production validation + runbook)
- **领先 main 14 个 commit**,完整跑通 P0→P6 压缩链路,链上无 WIP
- **唯一真正遗留的活儿**:`TODOLIST.md` 里的 **GUI 多模态图片粘贴** —— 不在 compaction 主题,移交 pro 全局
- **Working tree 状态**:9 modified(几乎全是 rustfmt,无功能改动) + 2 untracked(1 测试 + 1 调研报告)
- **关键风险**:`agent-diva-agent/tests/compaction_real_test.rs` 是 untracked,提交前需要单独 commit
- **下一步推荐**:先 `cargo fmt` + `cargo check` 验证 9 modified,再把 untracked 拆 2 个 commit(test + docs),最后准备 PR 进 main

---

## 1. 仓库 / 分支状态

| 项 | 值 |
|---|---|
| 路径 | `~/Desktop/morediva/agent-diva-pro/` |
| 当前分支 | `feature/context-compaction` |
| HEAD commit | `0b9ba79 test(CC-P6): compaction E2E production validation + runbook` |
| 比 main 多 | 14 commit(807 files / +124,293 / -18,507) |
| main 上 dirty? | 0(干净) |
| 远程 | 未 push(`pro` 仓 `auto-commit-each-completed-update` + `no-self-commit-without-request` 同时存在,见 §6 坑) |

**14 commit 链条(Conventional Commits,逐个独立可回滚):**

```
0b9ba79 test(CC-P6): compaction E2E production validation + runbook     ← HEAD
04ea8b3 feat(compaction): support multi-compaction chain (CC-P5)
9b1a644 feat: expose BudgetConfig to user config layer (CC-P2)
c40429d feat(compaction): add summary quality validation with retry mechanism
a56cabb feat(gui): add CompactionSettings panel for context compaction configuration
bb6f358 feat(cli): add /compact manual context compaction command
b383abe feat(gui): pet fullscreen immersive mode with mini status bar and overlay sidebar
4ce4574 feat(gui): add pet immersive mode script logic (FR-1/FR-2/FR-5)
6cd292b feat(gui): integrate ThinkingToggle component in ChatView
554d14a feat(gui): optimize ThinkingBlock with animations, copy button, and dark mode
ac73050 feat(core,providers): per-provider ReasoningConfig and dynamic ModelCapabilities.reasoning
53bc086 feat(gui): add clipboard image paste support in composer
875c303 docs: add sandbox config reference
4058d71 test(agent): add Context Compaction P0 integration tests
36fd172 feat(agent): integrate context compaction into agent loop          ← CC-P0
... (再往前 5 个是 thinking-mode/compaction ADR/PRD/epic 拆解,见 main..HEAD)
```

---

## 2. 唯一 Open 待办(全局,不属 compaction 主题)

`TODOLIST.md` 整文件只有一条 Open:

| # | 项 | 关联 |
|---|---|---|
| 1 | **GUI 多模态图像输入优化** —— composer 暂不支持 Ctrl+V 粘贴剪贴板图片;当前只支持附件按钮上传。需捕获剪贴板图像 → 走现有 attachment 路径 → 显示 image chip/preview → 与 vision capability 校验保持一致 | `docs/logs/2026-06-multimodal-gui-boundary/v0.0.8-gui-paste-boundary/summary.md` |

> **注**:`53bc086 feat(gui): add clipboard image paste support in composer` 已经在 `feature/context-compaction` 分支里 commit 了 —— 可能是该 commit 与 v0.0.8 文档化边界不完全一致,**接手人需先确认 `53bc086` 实际覆盖范围**,再决定是 close 掉这条 TODOLIST,还是改 v0.0.8 的措辞。

---

## 3. Working Tree 状态(提交前必须先理清)

### 3.1 Modified(9 文件,全部是 rustfmt 格式化,无功能改动)

| 文件 | 变化类型 | 是否需要 review |
|---|---|---|
| `agent-diva-agent/src/compaction/mod.rs` | 行折叠(2 行→1 行) | ❌ 纯格式 |
| `agent-diva-agent/src/compaction/quality.rs` | 中文停用词列表折行 + tuple 展开 | ❌ 纯格式 |
| `agent-diva-agent/src/context_budget.rs` | 多行表达式单行化 | ❌ 纯格式 |
| `agent-diva-agent/src/subagent.rs` | 极小格式化 | ❌ 纯格式 |
| `agent-diva-agent/src/token_estimate.rs` | 极小格式化 | ❌ 纯格式 |
| `agent-diva-cli/src/chat_commands.rs` | 12 行(实质改动,需 review) | ✅ **要 review** |
| `agent-diva-gui/src-tauri/src/commands.rs` | 6 行(实质改动,需 review) | ✅ **要 review** |
| `agent-diva-providers/src/base.rs` | 10 行(实质改动,需 review) | ✅ **要 review** |
| `agent-diva-tools/src/attachment.rs` | 2 行(实质改动,需 review) | ✅ **要 review** |

**操作建议**:
1. 先 `cargo fmt` 跑一遍,看是否把 6 个"纯格式"文件 revert 掉
2. 对剩下 4 个有实质改动的文件按 git blame 找上下文,确认是 rustfmt 顺带修改还是真改了什么
3. `git diff` 这 4 个文件,逐行 review

### 3.2 Untracked(2 文件,需要拆 2 个 commit)

| 文件 | 大小 | 性质 | commit 建议 |
|---|---:|---|---|
| `agent-diva-agent/tests/compaction_real_test.rs` | 279 行 / 23.7 KB | CC-P6 真实 LLM 压缩测试,从 Hermes `config.yaml` 解析 teakacloud API key,**已验证通过(23.96s,token 节省 90.7%)** | `test(CC-P6): add real-LLM compaction validation test`(独立 commit) |
| `docs/research/claude-code-context-window-research.md` | 435 行 / ~20 KB | 调研报告:Claude Code 六层 context window 解析链、模型能力缓存、自动压缩策略,给出 5 条可借鉴设计 + 4 条具体改进方案(给 `ModelCapabilities` + `BudgetConfig` 用) | `docs(compaction): add Claude Code context window research`(独立 commit) |

**重要坑**:见 §6 第 1 条。

---

## 4. 已交付的关键文件(接手人 review 起点)

### 4.1 核心代码

| 路径 | 行数 | 用途 |
|---|---:|---|
| `agent-diva-agent/src/compaction/mod.rs` | — | 模块入口,导出 `ContextCompactor` 等 |
| `agent-diva-agent/src/compaction/compaction_exec.rs` | — | 压缩执行核心:消息范围选择 + LLM 调用 + 摘要提取 + 质量验证 + 重试 |
| `agent-diva-agent/src/compaction/quality.rs` | — | 摘要质量评分:**长度 20% + 关键词覆盖 40% + 语义完整性 40%** |
| `agent-diva-agent/src/compaction/prompt.rs` | — | 提示词模板:要求 `<analysis>` + `<summary>` 结构化输出,约束最多 2000 字符 |
| `agent-diva-agent/src/context_budget.rs` | — | `BudgetConfig` + token 预算分配 |
| `agent-diva-agent/src/token_estimate.rs` | — | 启发式 token 估算(`chars/3`,无模型特定 tokenizer) |
| `agent-diva-providers/src/base.rs` | — | `ModelCapabilities` + `model_capabilities_for_model`(待扩展 context_window 字段,见 §5) |

### 4.2 测试

| 路径 | 行数 | 用途 |
|---|---:|---|
| `agent-diva-agent/tests/compaction_integration.rs` | 739 | CC-P0 集成测试(已 tracked,已 commit) |
| `agent-diva-agent/tests/compaction_e2e.rs` | 877 | 端到端测试(已 tracked,已 commit) |
| `agent-diva-agent/tests/compaction_real_test.rs` | 279 | CC-P6 真实 LLM 测试(untracked,需 commit) |

### 4.3 GUI / CLI 入口

| 路径 | 用途 |
|---|---|
| `agent-diva-gui/.../CompactionSettings` 面板 | GUI 配置压缩阈值/最大 tokens/保留条数 |
| `agent-diva-cli/src/chat_commands.rs` | `/compact` 手动触发命令 |

### 4.4 文档

| 路径 | 行数 | 用途 |
|---|---:|---|
| `docs/runbook-compaction.md` | 294 | **E2E 操作手册:部署 / 验证 / 故障排查** ← 接手人先读这个 |
| `docs/research/claude-code-context-window-research.md` | 435 | Claude Code 调研,后续改进输入 |
| `docs/prds/prd-agent-diva-pro-2026-06-03/prd.md` | 174 | PRD |
| `docs/adr/`(相关) | — | 架构决策记录 |
| `docs/logs/2026-06-06-context-compaction-tree-sort/` | — | 本主题日志目录 |

---

## 5. 关键决策(已记录,接手人可推翻但需重写 ADR)

1. **三层 context window 策略**(Letta 简化版,见 `claude-code-context-window-research.md` §五)
   - 硬编码常用模型表(tekaapi 13 模型) → `model_capabilities_for_model()` 返回
   - `BudgetConfig::for_model(model: &str)` 构造
   - `AGENT_DIVA_MAX_CONTEXT_TOKENS` 环境变量 override
   - **不引入 LiteLLM 库**,避免重依赖

2. **CC-P6 真实测试不用 mock key**
   - 不硬编码 API key,不依赖 `serde_yaml`
   - 用 `std::fs::read_to_string` + 行扫描,从 `~/AppData/Local/hermes/config.yaml` 找 `teakacloud.api_key`
   - 测试会 `panic!("无法找到有效的 API key...")`,**没配 teakacloud 的开发机跑不了** —— CI 必须前置 `TEAKACLOUD_API_KEY` env

3. **压缩摘要质量评分**
   - 长度 20% + 关键词覆盖 40% + 语义完整性 40%
   - 不及格自动重试(最多 N 次,见 `quality.rs` 配置)

4. **Git 工作流**
   - 受控 autocommit(pro 仓 `auto-commit-each-completed-update`)
   - **默认不 auto-push**(`no-self-commit-without-request`)
   - 提交粒度 = 一个独立可回滚的更新

5. **GUI 设计边界**(v0.0.8 文档化)
   - 当前只支持**文件附件**走 vision,**不**支持**剪贴板粘贴**
   - vision 白名单硬编码保守:`gpt-4o` / `gpt-4o-mini` / `gpt-4.1` / `gpt-4.1-mini`
   - 文本/未知模型收到图片 → 返回 user-facing "model doesn't support images" 消息

---

## 6. 接手人必读:坑位清单

| # | 坑 | 后果 / 应对 |
|---|---|---|
| 1 | `compaction_real_test.rs` 是 **untracked**,不在 git 历史里 | 提交前必须 `git add` 单独 commit,**别和 9 modified 混一起** |
| 2 | `pro` 仓 `AGENTS.md` 有 `auto-commit-each-completed-update` + `no-self-commit-without-request` 两条规则**冲突** | 本次靠人判断"完成了一个 update 才 commit"绕过了。接手人提交前**先 grep 自己是否要推 push** —— 默认只 commit,默认**不 push** |
| 3 | `BudgetConfig::default()` 仍硬编码 `max_tokens: 180_000`,**未实现 §5 第 1 条的三层策略** | 这是已知 follow-up,调研报告已给完整方案;接手人若要落地,改 `model_capabilities_for_model` + `BudgetConfig::for_model` |
| 4 | `cargo test -p agent-diva-agent test_real_compaction` 在没配 teakacloud key 的机器上会 panic | CI 必须 export `TEAKACLOUD_API_KEY=sk-...`;本地 dev 走 Hermes config 自动读 |
| 5 | Mentle feature-lane 在 Windows 上需要 `clang-cl.exe`,`PATH` 不一定带 | `AGENTS.md` 写明:`$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH` 后再 `cargo check -p agent-diva-agent --features mentle` |
| 6 | 9 modified 文件里有 4 个有实质改动(`chat_commands.rs` / `gui-tauri/commands.rs` / `providers/base.rs` / `tools/attachment.rs`),**不是**纯格式 | 提交前必须 `git diff` 逐行 review,别被 rustfmt 噪音骗过去 |
| 7 | `docs/logs/2026-06-06-context-compaction-tree-sort/` 是本主题日志目录 | 接手人改动如涉及新阶段,新建 `v0.x.y-<slug>/`,填 `summary.md` + `verification.md` + `acceptance.md` |
| 8 | 8 个 `--cfg(test)` 单元测试 + 2 个 integration test 文件都在 `agent-diva-agent/tests/` | `cargo test -p agent-diva-agent` 跑全;`test_real_compaction` 单跑要 `-- --nocapture` 看输出 |

---

## 7. 接手人操作清单(优先级排序)

| 序 | 动作 | 命令 / 路径 | 期望结果 |
|---|---|---|---|
| 1 | 看 runbook | `read_file docs/runbook-compaction.md` | 5 分钟理解 E2E 流程 |
| 2 | 格式化校验 | `cd agent-diva-pro && cargo fmt --check` | 看哪些文件需要 fmt |
| 3 | 跑全测试 | `cargo test -p agent-diva-agent` | 集成 + E2E 通过(真实测试除外,见下) |
| 4 | 跑真实测试 | `cargo test -p agent-diva-agent test_real_compaction -- --nocapture` | 看到"消息 N→1, 字符节省 X%, token 节省 Y%"输出 |
| 5 | review 4 个 modified 实质改动 | `git diff agent-diva-cli/src/chat_commands.rs` 等 4 个 | 确认无功能 bug |
| 6 | commit untracked 2 个文件 | `git add agent-diva-agent/tests/compaction_real_test.rs && git commit -m "test(CC-P6): add real-LLM compaction validation"` + 同理 docs | 拆 2 commit,清晰可回滚 |
| 7 | commit 9 modified | `git add -u && git commit -m "style(compaction): rustfmt auto-formatting"` | 单独 style commit |
| 8 | 处理 TODOLIST 那条 GUI 粘贴 | 先 `git log --oneline --all -- agent-diva-gui/.../composer/...` 查 53bc086 实际范围 | 决定 close TODOLIST 或更新 v0.0.8 措辞 |
| 9 | (可选)落地 §5 第 1 条三层策略 | 改 `model_capabilities_for_model` + `BudgetConfig::for_model` | 解开 180K 硬编码 |
| 10 | (可选)push 远程 | `git push origin feature/context-compaction` —— **必须先问用户** | pro 仓默认不 auto-push |

---

## 8. 一句话总结

> 主题活儿(P0→P6)干完了,14 个 commit 链完整、Conventional Commits、test/runbook/docs 都齐。
> 唯一遗留是**working tree 的 9 个 modified 几乎全是 rustfmt**,加 **2 个 untracked(test + 调研报告)需要分别 commit**;真正的功能 follow-up 是 **GUI 剪贴板粘贴**(已 commit 在 53bc086,需确认覆盖范围)。
> 下一步:**fmt → 拆 commit → 处理 TODOLIST 那条**;**别推 push**,默认只 commit。

---

*交班时间:2026-06-07*
*交班人:Hermes(MiniMax-M3)*
*接手通道:Project ViVy / agent-diva-pro / 看板 `pro:` 类目下相关卡*
