# 沙箱安全审计精华（压缩版）

> 原始：6 篇审计文档（A/B/C + checklist + verification + files-map）
> 核心结论：29 项关键能力中 10✅ 12⚠️ 7❌，7 项高危 P0 缺失

---

## 1. 审计范围

- **目标**：agent-diva-sandbox v0.4.9，15 个 Rust 核心文件
- **方法**：提取 74 项安全能力清单，逐项与沙箱代码交叉核查

## 2. 关键发现

### 2.1 已完整实现（10 项）

1. 三层 Shell 命令黑名单 + 5 阶段审批编排
2. 命令参数过滤 + 工作区限制
3. 双系统文件访问控制 + OS 级强制
4. 8 层路径遍历防护纵深
5. 平台沙箱：Linux Landlock+Bwrap+Seccomp / macOS Seatbelt
6. 迭代预算控制（max_iterations=20）
7. 审批缓存三级决策（Denied/ApprovedOnce/ApprovedForSession）
8. 子Agent 递归深度控制
9. MCP 工具短路保护
10. 文件冲突检测（五层：版本号+SQLite+TOCTOU+断路器+限流）

### 2.2 高危缺失（7 项 P0）

| # | 缺陷 | 风险 | 修复难度 |
|---|------|------|----------|
| 1 | MCP 环境变量全量透传 | 恶意 MCP 读取宿主机 API key | 低（白名单过滤） |
| 2 | MCP 请求大小无限制 | 大请求 OOM | 低（一行大小检查） |
| 3 | Prompt 注入扫描缺失 | 恶意指令注入 | 中（正则+LLM 双重） |
| 4 | 威胁模式扫描（记忆写入） | 持久化注入攻击 | 中（参考 Hermes） |
| 5 | 子Agent 并发无限制 | 资源耗尽 | 低（max_concurrent） |
| 6 | Windows 沙箱存根 | 无实质隔离 | 高（需 Windows API） |
| 7 | 隔离区扫描缺失 | 下载插件无安全检查 | 中（四步流水线） |

### 2.3 部分实现（12 项）

- 凭证日志脱敏：CLI config show 有，日志层无全局过滤
- 断路器：仅审批拒绝维度，无工具执行失败熔断
- 流式中断恢复：仅 250ms keepalive，无连接级重试
- 空响应/幽灵动作检测：仅空 final_content 兜底
- 子Agent 工具黑名单：仅 allow/deny 标记，未联动运行时
- 子Agent 超时：仅外层 tokio::timeout，无诊断转储
- 子Agent 凭证最小化：无选择性传递机制
- 健康检查：仅 CLI ping，无结构化端点
- 审计日志：仅 tracing span，无持久化
- 进程资源限制：仅超时，无 CPU/内存限制
- 网络出站控制：macOS/Linux 已实现，Windows 缺失
- 平台沙箱：Linux/macOS 完整，Windows 存根

## 3. 修复优先级

1. **立即**：MCP env 白名单、子Agent 并发限制、Prompt 注入扫描
2. **短期**：日志全局脱敏、断路器完整实现、记忆写入安全扫描
3. **中期**：Windows 沙箱、隔离区扫描、健康检查端点

## 4. 原始文档

- `agent-diva-main/docs/dev/archive/awesomeagents/sandbox-audit-{a,b,c}.md`
- `agent-diva-main/docs/dev/archive/awesomeagents/sandbox-audit-checklist.md`
- `agent-diva-main/docs/dev/archive/awesomeagents/sandbox-verification.md`
