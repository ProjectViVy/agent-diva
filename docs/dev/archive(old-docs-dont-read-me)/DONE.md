# Archive Governance — DONE Items

> 生成日期：2026-06-13
> 范围：`docs/dev/archive(old-docs-dont-read-me)` 全量治理
> 原则：已完成的决策、已合并的分支、已关闭的 closeout 只留索引，不留全文

---

## 一、已完成的分支合并

### 1. main → agent-diva-pro 合并 (2026-06-02)

- **方案**：stash → force-merge → reapply
- **结果**：main 后端代码完整合并到 pro，pro 独有功能（usage 模块、token_stats）通过补丁保留
- **关键冲突解决**：
  - `agent-diva-core/src/lib.rs` — 添加 `pub mod usage`
  - `agent-diva-manager/src/lib.rs` — 同时保留 `token_stats` + `file_service`
  - GUI 层以 pro 为基础，集成 main 的文件上传 UI
- **原始文档**：`merge-main-to-pro-plan.md`（已归档，本文件替代）

### 2. MOREDIVA 分支归并 (2026-06-07)

- **决策**：AutoDream = 压缩层（生日报/周报 → `.agent-diva/autodream/reports/`）
- **决策**：Report System = 呈现层（月报 + 固化 + 搜索 + NotebookView）
- **commit**：d306f74，方案 C 边界分工
- **原始文档**：`MOREDIVA-分支归并决策.md`、`MOREDIVA-分支归并待办卡.md`（已归档）

---

## 二、已完成的 Closeout

### MAIN-CLOSE-01 Runtime Safety Baseline (done)

- 断路器、loop guard、context budget、overflow retry、subagent runtime guardrails、tool timeout
- 证据：`docs/logs/2026-06-agent-loop-safety/`、`docs/logs/2026-06-tool-timeout/`

### MAIN-CLOSE-02 Session Truth Source (done)

- 后端 session 持久化、restore 语义、runtime state 一致性
- 证据：`docs/logs/2026-06-session-truth-source/`

### MAIN-CLOSE-03 Logging, Redaction, Provider Safety (done)

- 敏感值脱敏、provider/runtime 错误上下文、filesystem/tool 安全调整
- 证据：`docs/logs/2026-06-log-redaction/`、`docs/logs/2026-06-observability/`

### MAIN-CLOSE-04 Backend Multimodal Boundary (done)

- 后端 attachment/request boundary、embedded gateway/server wiring
- 证据：`docs/logs/2026-06-multimodal-m1-contract/`、`docs/logs/2026-06-multimodal-prephase/`

### MAIN-CLOSE-05 Backlog and Documentation Closeout (done)

- TODOLIST.md 指向 closeout plan，前端/产品文件明确标记为 outside main

---

## 三、已完成的调研与决策

### 7 大 Agent 项目对比调研 (2026-06-01)

- **范围**：GenericAgent、Hermes、OpenHarness、Claude Code、Codex CLI、openfang、agent-diva
- **产出**：
  - 6 篇项目深度分析（genericagent/hermes/claude-code/openharness/codex/other-projects）
  - 功能对比矩阵（7 项目 × 8 维度）
  - 27 个未知缺陷分析
  - 24 个能力缺失项
  - 最终决策记录（P0/P1/P2 分级）
- **状态**：调研完成，决策已定，进入执行阶段
- **精华保留**：`RESEARCH-精华/awesomeagents-comparison.md`（压缩版）

### 沙箱安全审计 (2026-06-02)

- **范围**：agent-diva-sandbox v0.4.9，15 个 Rust 核心文件
- **结果**：
  - 维度 A（隔离+Shell）：5✅ 3⚠️ 1❌
  - 维度 B（凭证+注入+熔断）：2✅ 4⚠️ 3❌
  - 维度 C（子Agent+MCP+审计）：3✅ 5⚠️ 3❌
- **关键发现**：
  - Windows RestrictedToken 是 dead code
  - MCP 环境变量全量透传（高危）
  - 子Agent 并发无限制
  - 日志层无全局凭证脱敏
- **精华保留**：`RESEARCH-精华/sandbox-audit-summary.md`（压缩版）

### Pro UI 审计 (2026-06-02)

- **发现**：权限模式 UI 存在但前后端未连通
- **状态**：已识别，待修复

---

## 四、已废弃/替代的架构

| 旧架构 | 替代方案 | 原因 |
|--------|---------|------|
| 外部 gateway 进程（release mode） | 嵌入式 gateway（`127.0.0.1:0` 预绑定） | 简化部署，消除进程间通信 |
| `agent-diva-nano` 内嵌 monorepo | 外部独立仓库 | 解耦发布周期 |
| 扁平 MEMORY.md 全量注入 | 分层记忆（L1-L4，规划中） | 解决 token 浪费和检索效率 |
| 纯串行工具执行 | 只读并发 + 写入串行（规划中） | 性能优化 |

---

## 五、已完成的 Mentle 集成 Sprint

- S2 完成：published-crate 约束、interface baseline
- S3 完成：MemtleToolkitTool adapter、dynamic tool registration、error mapping、runtime assembly
- S4 完成：adapter/runtime compatibility、regression test baseline、build env
- S5 完成：failure validation matrix、CI hardening
- S6 完成：RC scope baseline、final functional acceptance、downgrade failure acceptance
- S7 进行中：tool selection + GUI controls
- **原始文档**：`mentle-integration/` 下 26 篇 sprint 文档（已归档）

---

## 六、已完成的 Nano 解耦

- `agent-diva-nano` 已外化为独立仓库
- crates.io 发布策略已定（`agent-diva-core` → ... → `agent-diva-manager` → `agent-diva-cli`）
- **原始文档**：`nano/` 下 8 篇文档（已归档）

---

## 七、已验证的 Bug 修复

### GUI Connection Issue (2026-03)

- **根因**：Windows HTTP proxy 拦截 localhost 请求
- **修复**：`reqwest::Client::builder().no_proxy()` + IPv4 绑定
- **文件**：`agent-diva-gui/src-tauri/src/app_state.rs`

### File Upload Path Mismatch (2026-03)

- **根因**：upload 用 `%LOCALAPPDATA%`、read 用 `~/.agent-diva`
- **修复**：统一使用 `dirs::data_local_dir()`
- **文件**：`agent-diva-agent/src/agent_loop/loop_turn.rs`

---

## 八、文档索引（原始文档位置）

所有原始文档仍保留在 `archive(old-docs-dont-read-me)/` 下，按以下结构：

```
archive(old-docs-dont-read-me)/
├── agent-diva-main/           # main 分支历史文档 (638 files)
├── agent-diva-pro-legacy/     # pro 分支 legacy 文档 (721 files)
├── agent-diva-pro-dev/        # pro dev audit (70 files)
├── awesomeagents/             # 7 项目调研原始文档
├── architecture-reports/      # 架构报告原始文档
├── memory-evolution/          # 记忆系统演进原始文档
├── mentle-integration/        # Mentle sprint 原始文档
├── nano/                      # Nano 解耦原始文档
├── sandbox-audit/             # 沙箱审计原始文档
└── ...                        # 其他专题原始文档
```

> **访问方式**：如需查阅原始全文，按上述路径进入对应子目录。

---

## 九、与当前主架构的冲突声明

以下旧架构/决策与当前 `agent-diva-pro` 主架构冲突，**不应再引用**：

1. **外部 gateway 进程模式** — 已替换为嵌入式模式
2. **`agent-diva-nano` 内嵌 monorepo** — 已外化为独立仓库
3. **前端/产品 UI 规划（main closeout 排除项）** — 已移至 pro 分支独立演进
4. **Mentle 全量集成计划（S1-S6）** — 已完成，S7+ 在 pro 分支继续
5. **旧版 provider 配置链** — 已重构为统一 Provider trait + LiteLLM 兼容
6. **旧版 skill 加载机制** — 已迁移至 SkillLoader + 缓存体系

---

## 十、已完成的决策（PD-02 ~ PD-14）

> 2026-06-13 由大湿确认完成，从 PENDING-DECISIONS.md 移入。

| 编号 | 事项 | 备注 |
|------|------|------|
| PD-02 | 自研 Skill 进化系统（周报提取） | 方向已定，独立 crate 方案 |
| PD-03 | 上下文压缩管线实现 | Phase 1.1 已完成 |
| PD-04 | 子 Agent 安全三件套 | 黑名单 + depth 控制已落地 |
| PD-05 | 记忆写入安全扫描 | 威胁模式扫描已实现 |
| PD-06 | 沙箱审批 UI 实现 | 前后端已连通 |
| PD-07 | 上下文健康状态行 | TokenBudget 指示已集成 |
| PD-08 | 分层记忆架构 | L1 索引层已落地 |
| PD-09 | 图像识别 Phase 2 | image_url 内容块已实现 |
| PD-10 | Mentle S7 范围 | S7 已完成，无 S8 |
| PD-11 | Nano 运行时打包策略 | crates.io + GitHub Release 并行 |
| PD-12 | Diva Pet 3D 背景集成 | 方案已定，资源到位后可开工 |
| PD-13 | Hermes 学习融合 | 已取消，专注自研 skill 进化 |
| PD-14 | Thin Observability Layer | tracing + 结构化日志已落地 |
