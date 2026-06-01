# Sandbox 审计报告 C：子Agent安全 + MCP安全 + 审计

> 审计目标：`agent-diva-sandbox/`
> 日期：2026-06-02

---

## 审计总览

| # | 维度 | 状态 | 说明 |
|---|------|------|------|
| 1 | 子Agent 工具黑名单 | ⚠️ | 有路径/扩展名黑名单，无 `delegate_task`/`memory`/`send_message` 级工具黑名单 |
| 2 | 子Agent 递归深度控制 | ✅ | `max_tool_iterations=20` + cron 递归防护 |
| 3 | 子Agent 并发线程限制 | ❌ | 仅 QQ channel 有 `max_concurrency`，无全局子Agent并发控制 |
| 4 | 子Agent 超时/运行时间限制 | ⚠️ | MCP 超时 30s 有，但无单次 agent turn 总超时 |
| 5 | 子Agent 凭证最小化 | ⚠️ | API key 非硬编码，但无子Agent凭证裁剪机制 |
| 6 | MCP 环境变量过滤 | ❌ | `config.env` 整体透传，无过滤 |
| 7 | MCP 请求大小限制 | ❌ | 未实现 |
| 8 | MCP 工具短路保护 | ✅ | `tokio::time::timeout` 包裹所有 MCP 调用 |
| 9 | 健康检查端点 | ⚠️ | `/api/health` 存在但仅返回 `"ok"`，无深度状态 |
| 10 | 审计日志 | ⚠️ | tracing + 滚动日志有，无结构化安全审计事件 |
| 11 | 文件冲突检测 | ✅ | Memory 版本号 + SQLite ON CONFLICT + TOCTOU 防护 |

**总结：✅ 3 项 | ⚠️ 5 项 | ❌ 3 项**

---

## 详细核查

### 1. 子Agent 工具黑名单 ⚠️

**现状：** 有路径和文件扩展名黑名单，但缺少针对子Agent可调用工具的细粒度黑名单。

**已实现：**
- `SecurityConfig` 定义了 `forbidden_paths`（`/etc`, `/root`, `~/.ssh`, `~/.aws` 等）和 `forbidden_extensions`（`.exe`, `.dll`, `.bat` 等）
- `PathValidator` 7 层路径验证强制执行

**缺失：**
- 无禁止子Agent调用 `delegate_task`、`memory`、`send_message` 等高权限工具的机制
- 子Agent继承父Agent全部工具权限，无降权

**文件：**
- `agent-diva-core/src/security/config.rs` — 黑名单定义
- `agent-diva-core/src/security/path.rs` — 路径验证器

**建议：** 引入 `subagent_denied_tools: Vec<String>` 配置项，在子Agent的 tool definitions 中过滤掉高权限工具。

---

### 2. 子Agent 递归深度控制 ✅

**现状：** 已实现两层防护。

**防护层 1 — 工具循环迭代上限：**
- `max_tool_iterations: 20`（默认），限制单条用户消息触发的最大工具调用轮次
- 文件：`agent-diva-core/src/config/schema.rs:89`

**防护层 2 — Cron 递归防护：**
- cron 触发的 turn 中自动过滤掉 `cron` 工具定义
- 即使工具调用漏过，执行层仍有硬拦截：`"Error: cron tool is disabled during cron-triggered execution"`
- 文件：`agent-diva-agent/src/agent_loop/loop_turn.rs:111-125, 291-292`

**补充：** glob 扫描深度限制 `glob_scan_max_depth: Some(10)`
- 文件：`agent-diva-sandbox/src/platform/linux.rs:54`

---

### 3. 子Agent 并发线程限制 ❌

**现状：** 无全局子Agent并发控制。

**仅有：** QQ channel 的 `max_concurrency` 字段（来自 QQ 网关 API，非框架级控制）
- 文件：`agent-diva-channels/src/qq.rs:150`

**缺失：**
- 无 Tokio semaphore 或类似机制限制同时运行的子Agent数量
- 无全局并发任务池
- 恶意或失控的子Agent可 fork 大量并发任务耗尽系统资源

**建议：** 在 agent loop 层引入 `Arc<Semaphore>` 限制并发子Agent数量，配置项 `max_concurrent_subagents`。

---

### 4. 子Agent 超时/运行时间限制 ⚠️

**现状：** MCP 工具调用有超时，但缺少 agent turn 级别的总超时。

**已实现：**
- MCP 默认超时 30s（`DEFAULT_TIMEOUT_SECS`）
- MCP 客户端启动超时 `clamp(10, 120)` 秒
- `list_tools()` 和 `call_tool()` 均包裹在 `tokio::time::timeout` 中
- 文件：`agent-diva-tools/src/mcp_sdk.rs:49, 196-204, 255-258`
- `tool_timeout` 最小值强制为 1 秒
- 文件：`agent-diva-manager/src/mcp_service.rs:229`

**缺失：**
- 无单次 agent turn 的总运行时间上限（`max_tool_iterations` 限制轮次但不限时间）
- 无子Agent级别的超时配置
- 一个子Agent可在 LLM 调用中长时间挂起

**建议：** 增加 `turn_timeout_secs` 配置，在 `loop_turn` 外层包裹 `tokio::time::timeout`。

---

### 5. 子Agent 凭证最小化 ⚠️

**现状：** 凭证通过配置传递，非硬编码，但无子Agent级别的凭证裁剪。

**已实现：**
- API key 存储为 `Option<String>`，从配置加载
- 文件：`agent-diva-agent/src/tool_config/network.rs`
- 四级安全策略（Permissive → Paranoid）可限制写操作

**缺失：**
- 子Agent继承父Agent全部凭证，无最小权限裁剪
- channel token/secret/password 明文存储在 config.json 中，无加密
- 文件：`agent-diva-core/src/config/schema.rs:200-391`（`token`, `app_secret`, `smtp_password` 等）

**建议：**
- 子Agent应仅接收其工作所需的最小凭证集
- 敏感凭证考虑使用 OS keyring 或加密存储

---

### 6. MCP 环境变量过滤 ❌

**现状：** `config.env` 整体透传给 MCP 子进程，无过滤。

```rust
// agent-diva-tools/src/mcp_sdk.rs:128-132
if config.env.is_empty() {
    None
} else {
    Some(config.env.clone())  // 全量透传
}
```

**风险：** 恶意 MCP 服务器可通过环境变量泄露宿主机密钥（`API_KEY`, `AWS_SECRET_ACCESS_KEY` 等）。

**仅有限防护：** sandbox policy 有 `exclude_tmpdir_env_var` 布尔值，仅排除 TMPDIR。
- 文件：`agent-diva-sandbox/src/policy.rs:46`

**建议：**
- 默认使用白名单模式，仅传递显式声明的环境变量
- 提供 `mcp_env_deny_list` 配置项过滤敏感变量

---

### 7. MCP 请求大小限制 ❌

**现状：** 未发现任何 MCP 请求/响应大小限制。

**缺失：**
- 无 `max_request_size` / `max_response_size` 配置
- 无 JSON payload 大小校验
- 恶意 MCP 服务器可返回超大响应耗尽内存

**仅有的防护：** `sanitize_json_strings()` 清理控制字符，但不检查总大小。
- 文件：`agent-diva-tools/src/mcp_sdk.rs:26-45`

**建议：** 在 `call_tool()` 中增加响应大小检查，超过阈值（如 10MB）截断或报错。

---

### 8. MCP 工具短路保护 ✅

**现状：** 已实现多层超时短路。

**防护层：**
1. MCP 客户端启动超时：`clamp(10, 120)` 秒 → `McpError::Timeout`
2. `list_tools()` 超时包裹
3. `call_tool()` 超时包裹（默认 30s）
4. 类型化错误：`ProcessStart`, `ConnectionFailed`, `Timeout`, `Sdk`, `Server`, `Config`

**文件：** `agent-diva-tools/src/mcp_sdk.rs:196-258`

**评价：** 防护完善，不会因 MCP 服务器挂起导致 agent 阻塞。

---

### 9. 健康检查端点 ⚠️

**现状：** 端点存在但功能极简。

**已实现：**
- `GET /api/health` → 返回 `"ok"`（liveness probe）
- 文件：`agent-diva-manager/src/handlers.rs:733`，`server.rs:174`
- GUI 轮询该端点（1s 超时）
- 文件：`agent-diva-gui/src-tauri/src/commands.rs:1053`
- 嵌入式服务器启动后轮询最多 30 次
- 文件：`agent-diva-gui/src-tauri/src/embedded_server.rs:191`
- HeartbeatService 提供 `status()` 方法返回 JSON 状态
- 文件：`agent-diva-core/src/heartbeat/service.rs:154`

**缺失：**
- 无 readiness probe（不检查依赖是否就绪）
- 不报告：活跃 session 数、内存使用、provider 连接状态、MCP 服务器状态
- 无 `/api/status` 或详细诊断端点

**建议：** 扩展为包含 `sessions_active`, `providers_connected`, `mcp_servers`, `memory_mb` 的 JSON 响应。

---

### 10. 审计日志 ⚠️

**现状：** 基础日志设施完备，但缺少结构化安全审计事件。

**已实现：**
- `tracing` 框架 + `tracing-subscriber`
- 日志级别可按模块配置
- JSON 或文本格式可选
- 每日滚动日志文件 `gateway.log.YYYY-MM-DD`
- 自动清理 7 天前日志
- 非阻塞写入
- 文件：`agent-diva-core/src/logging.rs`
- Guardian 断路器触发时记录 `warn!`
- 文件：`agent-diva-sandbox/src/guardian.rs:440`
- 规则有 `justification` 审计字段
- 文件：`agent-diva-sandbox/src/rules.rs:22`
- 文件删除返回被删条目用于审计
- 文件：`agent-diva-files/src/manager.rs:369`

**缺失：**
- 无专门的安全审计日志流（如 `SECURITY_AUDIT` target）
- 工具调用不记录：谁调用了什么工具、传入了什么参数、返回了什么结果
- 无审计日志的独立存储和不可篡改性保证
- 无审计事件查询 API

**建议：**
- 引入 `audit::log_event(action, actor, target, result)` 统一接口
- 安全事件写入独立的 `audit.log` 文件
- 工具调用记录入参摘要（脱敏后）和结果状态码

---

### 11. 文件冲突检测（并行写入保护） ✅

**现状：** 已实现多层冲突防护。

**防护层 1 — Memory 版本号（乐观锁）：**
- `Memory.version: u64` 字段，每次 `update()` 自增
- 文件：`agent-diva-core/src/memory/storage.rs:13, 37`

**防护层 2 — SQLite ON CONFLICT：**
- 文件索引使用 `ON CONFLICT(id) DO UPDATE SET` 处理并发插入
- 文件：`agent-diva-files/src/index.rs:242`

**防护层 3 — TOCTOU 安全验证：**
- `validate_parent_directory()` 先创建目录再 canonicalize，防止符号链接攻击
- 文件：`agent-diva-core/src/security/policy.rs:203`

**防护层 4 — Guardian 断路器：**
- 连续拒绝超过阈值后阻断自动批准，防止拒绝风暴
- 文件：`agent-diva-sandbox/src/guardian.rs:400`

**防护层 5 — 滑动窗口限流：**
- `ActionTracker` 基于时间窗口限制操作频率
- 文件：`agent-diva-core/src/security/rate_limit.rs`

**评价：** 冲突检测体系完整，覆盖内存、数据库、文件系统三个层面。

---

## 修复优先级建议

| 优先级 | 项目 | 工作量 |
|--------|------|--------|
| P0 | MCP 环境变量过滤（#6） | 小 |
| P0 | MCP 请求大小限制（#7） | 小 |
| P1 | 子Agent并发限制（#3） | 中 |
| P1 | 子Agent工具黑名单（#1） | 中 |
| P1 | 子Agent超时（#4） | 小 |
| P2 | 结构化审计日志（#10） | 中 |
| P2 | 凭证最小化（#5） | 大 |
| P2 | 深度健康检查（#9） | 小 |
