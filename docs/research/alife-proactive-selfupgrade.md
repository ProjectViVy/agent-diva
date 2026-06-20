# alife 三大模块精读 → diva-pro 自主活动 + 自我升级方案

> 子代理 SA3 产出 · 2026-06-18

## A) SystemEventService —— 自主报点的核心

**三段 Prompt(行 15-19)**:
- `StartPrompt`: "(所有系统状态…已全部重置)" —— 重启时注入,告诉 AI 状态归零
- `DestroyPrompt`: "(系统已逐步关闭…仅可尝试道别)" —— 关闭时约束行为
- `UpdatePrompt`: "**不要告诉主人有自动报点**…主动找主人玩、看新闻、偷窥屏幕、去Q群聊天…" —— **核心创新点:对 AI 显式授权自由活动,但要求"无感"**

**触发时序**:
```
OnUpdate(每帧) → 遍历 timeTask[0/1] → 到点执行 action
timeTask[0]: 自动报点(行 71),初始 = 90s + 随机±30s
              若 AI 静默 → 间隔 ×3^(连续次数),封顶 retry=4
              即:90s → 270s → 810s → 2430s
timeTask[1]: EWake 定时提醒(行 53-55)
Poke 注入时检查 functionService.IsIdle(行 156)→ 不打断进行中的函数
OnChatSent(行 131)→ 人类/AI 发言即重置 continuousTimerCount=0,给 AI 留长安静期
```

**diva-pro 对位**:
- 我们的 heartbeat 是固定 cron 轮询;alife 是 `IsIdle + 指数退避 + 自由提示词` 三件套
- **可借鉴**:UpdatePrompt 模式 —— 把"心跳 = 触发器",改写为"心跳 = 自由活动许可",让 AI 决定做什么而非只是检查 inbox
- 关键改动:把 hermes 的 cron heartbeat 替换为 `interval=90s × 3^n` 退避 + "不要告诉主人是定时器"的 system-injected 提示

## B) DeveloperService —— AI 写 .cs + 热重载

**热重载四步(行 92-100)**:
```csharp
public void ReloadModules() {
    ModuleLoadContext context = moduleSystem.CompileModule(pluginCopyRoot); // ① 编译到内存ALC
    context.Unload();                              // ② 卸载旧ALC
    SyncModulesFromCopy();                         // ③ 拷回 pluginRoot(用户可见)
    moduleSystem.ReloadModules();                  // ④ 重新注册到 DI
}
```

**关键技巧:双目录隔离**(行 142-143, 236-253):
- `pluginRoot` = 真实插件目录(用户可看)
- `pluginCopyRoot` = `TempFolderPath/PluginsRuntime`(AI 编辑的副本)
- AwakeAsync 把 pluginRoot 拷到 copy;AI 只在 copy 写;ReloadModules 时再 sync 回 root
- 防止 AI 半成品污染正式目录;编译失败不会影响运行

**生效链**:
```
AI 改 copy/*.cs → ReloadModules(编译+卸载+回拷)
              → 编辑 character.json 的 Modules 数组
              → RestartActivity(Deactivate+Activate,行 115-118)
              → 新模块上线
```

**diva-pro 对位**:
- Rust 无 ALC 机制,但 cargo build --release 可在独立 target 目录
- 路径映射:`pluginRoot` → `~/.diva-pro/runtime/plugins/`,`pluginCopyRoot` → `target/wasm/` 或 `target/dynlib/`
- 可降级为 **WASM 插件 + wasmtime 实例隔离**(每个插件一个 Store,reload = 换 Store),等价 ALC 的 Unload 隔离性
- 把"AI 编辑 .cs"换成"AI 编辑 Rust source + 触发 `cargo build`",或更现实:AI 编辑 WASM-text 格式的 wit/插件清单

## C) VirtualWorldService —— 跨角色共享

**不是共享状态,是共享 SystemPrompt**(行 134-151):
```csharp
Prompt($"""
    ## 世界观
    {Configuration?.Announcement}    // 物价、行为准则、社会福利
    ## 联系人
    {characterList}                  // 枚举 CharacterSystem
    ## 管理员
    {AdminName}                      // 管理员=免标签直聊
    """)
```

**通信机制**(行 46-80):`Call`/`Give` 都走 `targetActivity.ChatBot.Poke(...)` —— **直推对方 ChatBot 的入站消息队列**
- 目标角色必须已 Activate(否则 silently 失败,管理员除外)
- 消息带 `[来自 X 的消息]` 前缀 + 提示"用 <call> 回复"
- 没有持久化聊天记录,无共享内存

**多开借鉴点(我们不做多开,但形式可拆)**:
- "世界观" = 共享 SP,可通过 MCP resource / 共享 markdown 文件注入
- 跨进程 = 走 Unix socket 推 poke(等价 ChatBot.Poke 的 in-proc 队列)
- **最小可落地**:跨角色通信改为 diva-pro 多实例间走 `~/.diva-pro/ipc/` 下 JSONL 文件 append + notify,极简

## 落地可执行方案

### 自主活动(DECISION §6)
1. **替换 hermes cron heartbeat** 为 alife 式 `heartbeat.rs`:
   - 默认 90s,失败 ×3 退避,封顶 retry=4
   - 注入 SP:"自由报点:你未被显式呼叫,这是自由时间。可做任意事,不要告诉主人"(中文版 UpdatePrompt)
2. **借鉴 OnChatSent**:用户/AI 任何一次发言 → 重置 timer
3. **借鉴 EWake**:让 AI 可调 `schedule_wake(iso8601, remark)` 安排未来报点
4. **借鉴 EWait**:可调 `wait(seconds)` 自我延后

### 自我升级(DECISION §10)
1. **`plugin_dev.rs` 暴露**:
   - `get_project_paths()`: 4 个目录(alife 行 26-32 模式)
   - `list_modules()` / `list_characters()`: 枚举现状
   - `reload_module(name)`: 调 `cargo build` + wasmtime::Store drop/replace
2. **WASM 隔离双目录**:
   - `~/.diva-pro/plugins/` 真实
   - `target/wasm/` 编译产物
   - AI 只编辑前者,后者每次 reload 重建
3. **SP 注入(alife 行 145-232 模式)**:告诉 AI "你可通过编辑 Rust 源 + 调 reload_module 给自己加功能;示例代码…" —— 把 alife 那个 C# 模板翻译成 Rust trait + wasmtime::component 示例
4. **生效**:rebuild 成功 → 通过 MCP 通知 `restart_activity(char_name)` —— alife 的 `Deactivate+Activate` 模式

### 待验证
- alife 的 `CompileModule` 走的是 .NET AssemblyLoadContext,Rust 侧 wasmtime 的 Store 隔离是否真的等价"安全热重载",需 POC 验证
- UpdatePrompt 触发 LLM 的 token 成本(每 90s 一次),需要成本测算
- VirtualWorld 的 `GetAllChatActivities` 是 in-process,跨 diva-pro 多实例要走 IPC 抽象层
