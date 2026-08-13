---
title: "Alife Harness 能力研究 — diva Harness v2 借鉴源"
date: 2026-06-20
status: research baseline (大湿对话整理, 2026-06-19-20 实测)
owner: 大湿
applies_to: agent-diva-pro
supersedes: null
related:
  - docs/research/harness-engineering-three-way-comparison.md          # 13 维度三方对比 (220 行)
  - docs/research/harness-engineering-three-way-detailed-checklist.md   # 21 维度详细 checklist (324 行)
  - docs/research/hermes-harness-overview.md                            # 兄弟文档 (hermes 视角, 456 行)
  - DECISION-v2-next-phase.md                                          # Harness Engineering 决策 (411 行)
source_of_truth:
  - alife 代码 (本次深读): ChatBot.cs (308 行) / ModuleSystem.cs (356 行) / InteractiveModule.cs (103 行) / ConfigurationSystem.cs (65 行) / MemoryManager.cs (251 行) / SkillService.cs (71 行)
  - alife 代码 (辅助, 之前研究已覆盖): SystemEventService.cs (168 行) / DeveloperService.cs (279 行) / MemoryStorage.cs (195 行) / TextVectorizer.cs (84 行) / HistoryCompressor.cs (7 行)
research_method: alife 走深读 (核心 6 文件 + 辅助 5 文件, 总 1887 行)
research_date: 2026-06-19 至 2026-06-20
research_perspective: 把 alife 当主角, 看 alife 自身有哪些 harness 能力, 作为 diva Harness v2 借鉴源
revision_notes:
  - v0.1 (2026-06-20) — 初版: 10 类 harness 能力盘点 + 与 diva 对位 + DECISION-v2 §5 对应清单 + 关键反面教材 (死锁 + 自我升级)
superseded_by: null
---

# Alife Harness 能力研究 — diva Harness v2 借鉴源

> **TL;DR**: Alife (C#/.NET 8 + SemanticKernel) 是单 ChatBot 实现的 AI agent 框架, diva 完成度 41% / alife **36%**. Alife 在 **Module 三段式生命周期 / Autofac DI / ChatBot Semaphore 互斥 / 自我升级** 上是最规范的（教科书式）, 但在 **防失控 / 稳 prompt cache / 配置管理 / 防护层** 4 维度上几乎空白. **Alife 死锁风险 (WPF 主线程) 是 diva 用 tokio 重新设计时必须避免的反面教材**.

---

## 0. 一句话哲学

> **Alife 把 agent 当"模块装配体"对待** — Module 三段式生命周期 + Autofac DI + Roslyn 热编译. 核心创新是"AI 主动写代码自我升级" (`DeveloperService`), 风险极大, **diva 不应照搬**. diva 必须从 alife 借鉴的是 Module/DI 的**规范度**, 同时**避开它的死锁风险和自我升级陷阱**.

---

## 1. 核心 Runtime — ChatBot 状态机

### 1.1 ChatBot 核心 (`ChatBot.cs` 308 行)

**8 个细粒度事件** (`event Func<...>?` / `event Action<...>?`):

| 事件 | 类型 | 触发时机 |
|------|------|---------|
| `PokeSend` | `Func<string, string>` | Poke 消息过滤（多层 Func 链） |
| `ChatSend` | `Func<string, string>` | 用户消息过滤 |
| `ChatSent` | `Action<string>` | 消息发送前 |
| `ChatReceived` | `Action<string>` | 流式 chunk 接收到 |
| `ReasoningReceived` | `Action<string>` | thinking 块接收到 |
| `ChatOver` | `Action` | 单轮 chat 结束 |
| `ChatHistoryAdd` | `Action<ChatMessageContent>` | history 追加 |
| `TokenUsed` | `Action<ChatTokenUsage>` | **token 消耗实时 hook** |

> **关键**: 跟 DECISION-v2 §5.8 Poke 8 事件链天然对位 — alife 8 事件命名就是 diva 借鉴源.

**流式响应** (`ChatStreamingAsync`):
- `ChatCompletionAgent.InvokeStreamingAsync()` + `IAsyncEnumerator<AgentResponseItem<StreamingChatMessageContent>>`
- 前置报文 `__THINK__` 前缀识别 thinking 块
- 元数据双源提取 (`ReasoningContent` + `Usage`)

### 1.2 SemaphoreSlim(1, 1) 互斥

```csharp
public async Task RequestChatAsync(CancellationToken cancellationToken = default) {
    await chatSemaphore.WaitAsync(cancellationToken);
}
public void ReleaseChat() { chatSemaphore.Release(); }
public bool IsChatting => chatSemaphore.CurrentCount == 0;
```

新消息打断上一次聊天 (`chatBreakSource.CancelAsync`).

### 1.3 PeriodicTimer 1s tick（`Update()` async void）

```
- 每 1s tick (currentTime += 1)
- if (currentTime - lastAutoFlushTime > 2) → TryFlushMessageCache
- lastAutoFlushTime = currentTime
```

### 1.4 IAsyncDisposable + 死锁修复

`DisposeAsync` 注释（行 228-245）作者承认 WPF 主线程 + 信号量会死锁, 需 `await Task.Yield()` 才能解. 见 §11.1 详细反面教材.

---

## 2. Module / DI — Alife 最强项

### 2.1 Module 三段式生命周期 (`InteractiveModule.cs` 103 行)

```csharp
public abstract class InteractiveModule : ISystemEvent {
    protected Character Character { get; private set; } = null!;
    protected ChatActivity ChatActivity { get; private set; } = null!;
    protected ChatBot ChatBot { get; private set; } = null!;

    public virtual Task AwakeAsync(AwakeContext context) {
        Character = context.Character;
        ChatHistory = context.ContextBuilder.ChatHistory;
        return Task.CompletedTask;
    }
    public virtual Task StartAsync(Kernel kernel, ChatActivity chatActivity) {
        ChatActivity = chatActivity;
        ChatBot = chatActivity.ChatBot;
        if (this is ITimeIterative interactiveModule) {
            updateCancellation = new CancellationTokenSource();
            StartUpdate(interactiveModule, updateCancellation.Token);
        }
        return Task.CompletedTask;
    }
    public virtual Task DestroyAsync() {
        if (updateCancellation != null)
            return updateCancellation.CancelAsync();
        return Task.CompletedTask;
    }
}
```

**3 段严格分离**:
- **Awake** — 接收 Character + ContextBuilder.ChatHistory（无副作用绑定）
- **Start** — 接收 Kernel + ChatActivity + ChatBot（实际启动）
- **Destroy** — 取消所有 CancellationToken（优雅退出）

**ITimeIterative 自动启动**（StartAsync 内部检查 is 检查） — 实现模块级定时循环的标准接口.

**`InteractiveModule<T>` 子类** 提供 5 个 helper (`Poke` / `Chat` / `ChatAsync` / `ImplicitChatAsync` / `Prompt` / `Throw`), 自动加 `[{T.Name}]` 前缀 + ChatTextFilter wrap.

### 2.2 ModuleSystem 反射 + Roslyn 热编译 (`ModuleSystem.cs` 356 行)

**AssemblyLoadContext 隔离**（自实现 `ModuleLoadContext: AssemblyLoadContext`）:
- `isCollectible: true` 可回收
- `LoadDll` + `LoadUnmanagedDll` 走多目录搜索 + RID 路径
- `ReloadContext` 卸载旧 + 加载新 + 重扫 `[Module]` attribute

**Roslyn 热编译** (`CompileModule` 6 步):

```
1. 创建 compilingContext (managedExtraDirectories + unmanagedExtraDirectories)
2. 扫描 *.dll → LoadDll 到 compilingContext
3. 扫描 *.cs → CSharpSyntaxTree.ParseText → 收集 references (去重)
4. CSharpCompilation.Create("Modules", syntaxTrees, references, Release优化)
5. compilation.Emit(dllPath, pdbPath) → 失败 throw 模块编译错误
6. compilingContext.LoadDll(tempModulePath)
```

**ReloadModules() 入口**:
- 确保 `Plugins` 目录存在
- `CompileModule(moduleRoot)` → `ReloadContext(...)` → `OnModulesReloaded` 事件

**[Module] attribute 注册**: `ReloadContext` 中扫 `assembly.GetTypes()` 找 `ModuleAttribute` 标记 + 非 abstract + 非 interface → `moduleTypes.Add(FullName, type)`.

### 2.3 ConfigurationSystem IConfigurable<T>（`ConfigurationSystem.cs` 65 行）

```csharp
public ConfigurationSystem(StorageSystem storageSystem) { ... }

public object? GetConfiguration(Type target, string root = "") {
    Type? configurationType = GetConfigurationType(target);
    // GetConfigurationType 反射找 IConfigurable<T> 接口的 T
    // 然后从 storageSystem.GetObject<JObject>(path) 反序列化
    // 没找到 → Activator.CreateInstance(configurationType) 用默认 ctor
}

public void SetConfiguration(Type target, object configuration, string root = "") {
    storageSystem.SetObject(Path.Combine(root, "Configuration", target.FullName!), configuration);
}
```

> **教科书式**: Module 自动注入 ConfigurationType, dev 改 JObject 文件, Awake 时 `context.GetConfiguration(...)` 拿到实例. **diva DECISION-v2 §5.1 的 Autofac DI 等价就该照搬这个模式** (用 `inventory` crate 替代 Roslyn 编译开销).

### 2.4 ModuleAttribute + DefaultCategory 分类

```csharp
[Module("Skill工具", "Skill 是一种渐进式...",
    defaultCategory: "Alife 官方/功能底座")]
public class SkillService : InteractiveModule<SkillService> { ... }
```

**Category 路径** (`defaultCategory` 用 `/` 分隔):
- `ReloadContext` 后 `SyncFolder()` 把 `moduleAttribute.DefaultCategory` 拆 path, 嵌套插入到 `StringFolder` 树
- 用户在 UI 可拖拽模块到不同文件夹（默认 + 自定义）

---

## 3. 记忆系统（4 文件）

### 3.1 MemoryManager 多 Level 压缩 (`MemoryManager.cs` 251 行)

**核心数据结构**:
```csharp
public record MemoryMeta(int Level, DateTime StartTime, DateTime EndTime);
public string Name => $"{Level}-{StartTime:yyyyMMddHHmmss}-{EndTime:yyyyMMddHHmmss}";
```

**多 Level Filter 循环**:
- 跳过系统提示词
- 遍历 chatHistory, 按 `MemoryMeta.Level` 划分 area
- `areaCompressionThreshold = areaLevel == 0 ? compressionThreshold : 4`
- `areaCount >= areaCompressionThreshold` → 压缩
- L0 → L1 → L2 → L3 → L4 链式压缩（最多 `maxCompressionLevel` 层）
- 压缩后插入 `[记忆存档({name})]` system message 到 chatHistory

**InsertMemory 定位**: 寻找同 level 区域最下方插入; 跨级时遵循"比目标 level 低的区域之前插入".

**SaveHistory / LoadHistory**: `historyStoragePath = $"{storagePath}/History.json"`, JSON 序列化 + 跳过 system prompt.

**Layered 索引**: 每个摘要记录描述涵盖范围 + 时间跨度, **比 diva `HybridMemoryProvider` 的 flat 模式强**.

### 3.2 MemoryStorage（`MemoryStorage.cs` 195 行）

- DuckDB + `array_cosine_similarity` 向量 + JSON 倒排索引
- `SearchAsync(level, keyword, question, count, offset, startTime, endTime)` 多维过滤
- **diva 无对应 DuckDB 实现**, 只用 SQLite FTS5 (在 hybrid.rs)

### 3.3 TextVectorizer（`TextVectorizer.cs` 84 行）

- 抽象 `TextVectorizer` + 多 backend: MiniCPM / Qwen / OpenAI
- diva 无此类抽象 (直接调 provider), **应作为 P2+ 候选**

### 3.4 HistoryCompressor（`HistoryCompressor.cs` 7 行）

接口定义, 实现未读. 跟 `MemoryManager.Filter` 配合压缩历史.

---

## 4. Skill 系统（单文件）

### 4.1 SkillService 渐进式披露（`SkillService.cs` 71 行）

```csharp
[XmlFunction(FunctionMode.OneShot)]
[Description("快速获取Skill信息")]
public void StudySkill(string name) {
    string skillDocPath = Path.Combine(skillsPath, name, "SKILL.md");
    if (File.Exists(skillDocPath) == false) {
        ChatBot.Poke($"[{nameof(StudySkill)}] skill文件不存在");
        return;
    }
    string skillDoc = File.ReadAllText(skillDocPath);
    string[] appendFiles = Directory.GetFiles(Path.Combine(skillsPath, name), "*", SearchOption.AllDirectories);
    Poke($"""
        [{nameof(StudySkill)}] 已读取 {name} skill
        > 包含文件：- {string.Join("\n- ", appendFiles)}
        > 手册内容：``` {skillDoc} ```
        """);
}
```

**关键设计**:
- **不在 startup 加载所有 skill**（节省 token）
- AI 主动调 `StudySkill("name")` → 读 SKILL.md → Poke 消息回灌
- SKILL.md 本身 = Markdown 手册, AI 读完按指导执行
- 跟 hermes §4.1 Skill 系统对位 — alife 是简化版 (单文件 vs hermes 多文件 + Hub 10 源)

### 4.2 AwakeAsync 注册 XmlHandler

```csharp
public override async Task AwakeAsync(AwakeContext context) {
    await base.AwakeAsync(context);
    string[] skills = Directory.GetDirectories(skillsPath).Select(Path.GetFileName).Cast<string>().ToArray();
    XmlHandler xmlHandler = new(this) {
        Description = "当你需要使用或管理Skill时调用。",
        Explanation = $"""已有 Skill: {string.Join("\n- ", skills)}"""
    };
    functionService.RegisterHandler(xmlHandler, DocumentMode.Implicit);
}
```

**DocumentMode.Implicit** = 显隐分桶（不在主文档中暴露, 按需触发）

### 4.3 配套工具

- `modelscope skills add` CLI 手动添加 skill
- 跟 hermes `Skills Hub` 10 源 + 6 层安全比, alife **只有 CLI 手动添加**, 无远程源 + 无安全机制
- diva 当前 `agent-diva-agent/src/skills.rs` 部分实接 (SkillsLoader + always field), 但无 alife 的 StudySkill 这种**渐进式披露 tool**

---

## 5. Poke 队列（核心创新 + DECISION-v2 §5.8 借鉴源）

### 5.1 Poke API（`ChatBot.cs:183-189`）

```csharp
public void Poke(string message) {
    while (messageCache.Count > 11)  // 队列上限 11 条
        messageCache.TryDequeue(out _);
    messageCache.Enqueue(message);
    lastAutoFlushTime = 0;  // 重新计时
}
```

### 5.2 自动 flush（`TryFlushMessageCache`）

- 2s 窗口内所有 Poke 合并为一条
- 走 `PokeSend` 过滤链（多层 Func<string, string>）
- `Chat($"{PokeMessageTag}\n{poke}")` 触发实际 chat
- `messageCache.Distinct()` 去重

**PokeMessageTag = "[来自系统的杂项消息推送]"** — 让 LLM 知道这是系统推的, 不是用户发的.

### 5.3 跟 DECISION-v2 §5.8 Poke 8 事件链对位

| Alife 事件 | diva 当前 | 借鉴方向 |
|-----------|----------|----------|
| `PokeSend` | ❌ 无 | pre_poke hook |
| `ChatSend` / `ChatSent` | ❌ 无 | message lifecycle hooks |
| `ChatReceived` | ❌ 无 | streaming chunk hook |
| `ReasoningReceived` | ❌ 无 | thinking block hook |
| `ChatOver` | ❌ 无 | turn-end hook |
| `ChatHistoryAdd` | ⚠️ MessageBus (粗粒度) | history sync hook |
| `TokenUsed` | ❌ 无 | **token 消耗 hook (实接 +5.8 P0)** |

> DECISION-v2 §5.8 已拍板"全拆不分期", alife 这 8 事件是**天然借鉴源**.

---

## 6. SystemEventService 自主报点（`SystemEventService.cs` 168 行）

**6 阶段循环**:

```
[1] Tick        — .NET PeriodicTimer 1s
[2] Check busy  — functionService.IsIdle
[3] Inject      — functionService.Poke() → ConcurrentQueue, 2s flush
[4] Wake        — LLM 看到 UpdatePrompt 消息
[5] Act         — AI 调工具 / 发消息 / 沉默
[6] Backoff     — 沉默 → 间隔 ×3^n 封顶 retry=4; 说话 → 重置回 90s
```

**EWake / EWait** (`timeTask` 2 槽位):
- `timeTask[0]` 自动报点定时器（指数退避）
- `timeTask[1]` EWake 自定义提醒

**UpdatePrompt 核心哲学**: "**不要告诉主人是定时器**" — 主动找主人玩 / 看新闻 / 自由活动.

**diva 借鉴方向**:
- ✅ DECISION-v2 §5.7 3 态用户存在 (Active/Distracted/Gone) 的灵感来源
- ✅ UpdatePrompt "主动活动许可" 哲学
- ❌ **不要照搬** alife 的单 90s 间隔 + 死锁风险 + 无 user presence

**已知缺陷** (per `autonomous-activity-thoughts.md` §3):
- 单一 90s 间隔（无 Distracted 10min 慢频 / Gone 30min 切换）
- **无 user presence 检测**（WPF Window.Activated / GetLastInputInfo 未用） — **diva 必须自己造**
- WPF 主线程 + 信号量 → **死锁风险**（见 §11.1）
- Poke 队列 2s flush

---

## 7. DeveloperService 自我升级（`DeveloperService.cs` 279 行）

> ⚠️ **风险极大**, diva 不应照搬. 但作为"AI 主动改自身"的参考, 值得了解.

**核心流程**:

```
1. AI 分析当前代码, 写 .cs 补丁
2. 调用 DeveloperService.SubmitPatch()
3. Roslyn 编译 .cs
4. AssemblyLoadContext 替换新 dll
5. ReloadModules() 重新加载所有 [Module]
6. 旧 Activity 卸载, 新 Activity 启动
```

**Alife 自己的风险** (per 三方对比 §12 段):
- AI 写自身 crate 代码 → 可能引入 bug 或安全漏洞
- 编译失败 → Activity 卡死
- 运行时崩溃 → 需要用户手动恢复

**diva DECISION-v2 §9 "警惕项" 明确**:
> "alife 自我升级(DeveloperService): 风险极大, Laputa 提案流程比这稳得多; 不要让 diva Harness 给 agent 写自身 crate 的代码"

---

## 8. Channel + UI + MCP

### 8.1 QChat 单通道 (`Alife.Function.QChat/`)

- OneBot 协议（QQ 机器人）
- **单通道架构**（vs diva 16 个 channel + hermes 14+ 平台）
- 适配 OneBot HTTP API + WebSocket

### 8.2 Alife.DeskPet Blazor + Live2D UI

- Blazor（.NET WebAssembly）+ Live2D 桌面宠物
- 异步更新 Razor 组件
- WPF / Razor 上下文传递（**死锁风险源**）
- diva 用 Tauri v2 + 不依赖单线程 UI 线程（已规避）

### 8.3 MCP 客户端 (`Alife.Function.Mcp/`)

- MCP 客户端实现（项目存在, 代码未深读）
- **diva 无 MCP 客户端**（per checklist §15: diva 0.5/8, hermes 8/8）
- DECISION-v2 未拍板, 应作为 P1+ 候选

---

## 9. Alife vs Diva Harness 能力对照（18 项）

| Alife harness 能力 | diva 状态 | 借鉴价值 |
|-------------------|----------|----------|
| 1. **Module 三段式** (Awake/Start/Destroy) | ❌ diva 无统一 trait | **DECISION-v2 §5.1 P0 待做 (主推)** |
| 2. **Autofac DI + IConfigurable<T>** | ❌ diva 无 DI 容器 | **DECISION-v2 §5.1 P0 待做 (主推)** |
| 3. **Roslyn 热编译 + AssemblyLoadContext** | ❌ diva 无插件热重载 | 推后到 v2+ (Plugin 路线 deferred, 用 WASM 替代) |
| 4. ChatBot Semaphore 互斥 (0/1) | ❌ diva 无 chat 互斥 | 借鉴, 但用 `tokio::sync::Semaphore` + 跨线程 spawn 避免死锁 |
| 5. **Poke 队列 + 8 事件** | ⚠️ diva MessageBus 粗粒度 | **DECISION-v2 §5.8 P0 待做 (主推)** |
| 6. SystemEventService 自主报点 + UpdatePrompt 哲学 | ⚠️ diva heartbeat 766 LOC | 借鉴 3 态用户存在 (DECISION-v2 §5.7 P0 待做) |
| 7. **DeveloperService AI 自我升级** | ❌ diva 严禁 (DECISION-v2 §9 警惕项) | 仅作"AI 主动改自身"反面教材 |
| 8. MemoryManager 多 Level 压缩 | ❌ diva 只有 L0/L1/L2 flat | 借鉴分层压缩 (`MemoryMeta(Level, StartTime, EndTime)`) |
| 9. TextVectorizer 抽象（MiniCPM/Qwen/OpenAI） | ❌ diva 无 | P2+ 候选 |
| 10. **Skill 渐进式披露 (StudySkill)** | ⚠️ diva `skills.rs` 部分实接 | 借鉴 alife 显隐分桶 + Poke 回灌 |
| 11. **Module 热重载** (ReloadModules + Roslyn) | ❌ diva 无 | 推后到 v2+ |
| 12. **ConfigurationSystem (IConfigurable 自动注入)** | ❌ diva 无 | **DECISION-v2 §5.1 P0 待做** |
| 13. 多通道 (QChat + DeskPet) | ✅ diva 16 channel 远超 | diva 强项 |
| 14. Tool 系统（XmlFunctionCaller 反射） | ✅ diva Rust trait-based 类型安全 | diva 强项 (Alife 落后) |
| 15. 错误恢复 (简单 try/catch) | ❌ diva 无 8 级分类 | 借鉴 hermes (Alife 没贡献) |
| 16. Prompt 注入防御 | ❌ diva 完全空白 | 借鉴 hermes §4.1 (Alife 完全无) |
| 17. 配置 hot reload | ❌ diva 无 | **DECISION-v2 §X-4 P0 待做 (Alife 用 storageSystem 部分借鉴)** |
| 18. Approval 系统 (1751 行) | ⚠️ diva 只有枚举 | 借鉴 hermes §5.1 (Alife 无, 但 InteractiveModule 三段式可借鉴为审批流) |

**完成度对照**: hermes **91%** / diva **41%** / alife **36%**

---

## 10. 跟 DECISION-v2 / Harness Engineering 对位

### 10.1 DECISION-v2 §5 P0 借鉴

| DECISION-v2 §5 P0 | Alife 对应能力 | 本研究来源 |
|------------------|----------------|-----------|
| #5.1 Module 生命周期 / Autofac DI | **§2.1 三段式 + §2.2 Roslyn + §2.3 IConfigurable** | 本研究主推 |
| #5.2 纯 Tool 硬约束 | (Alife 无 Tool, 用 hermes) | — |
| #5.5 行为审计 | §5.3 TokenUsed + ChatHistoryAdd 事件 | 本研究借鉴源 |
| #5.8 Poke 8 事件链全拆 | **§5.1 8 事件 (天然借鉴源)** | 本研究主推 |
| #X-4 Config hot reload | §2.3 ConfigurationSystem 用 storageSystem | 本研究借鉴源 |

### 10.2 DECISION-v2 §5 P1 借鉴

| P1 项 | Alife 对应能力 |
|-------|----------------|
| Toolset 分组 + check_fn | (Alife 无, 用 hermes) |
| Skill bundle + 多级 scope | §4 SkillService 但只有 CLI 添加, 无 bundle |

### 10.3 diva 已超过 Alife 的项（diva 强项）

| 维度 | diva | alife |
|------|------|------|
| 多通道 | 16+ channel | 1 (QChat) |
| Tool 系统 | Rust trait 类型安全 | XmlFunctionCaller 反射 (无类型) |
| Cron | `agent-diva-core/src/cron/` (1275 LOC) | 无 cron 抽象 |
| Sandbox | 5318 LOC (exec_policy + guardian + orchestrator) | 无沙箱 |
| 异步模型 | tokio 多线程 | WPF 主线程 (死锁风险) |

---

## 11. 关键反面教材（diva 必须避免的 Alife 坑）

### 11.1 死锁风险（最严重）

**Alife WPF 主线程 + Semaphore + 同线程 await = 死亡组合**.

作者承认（`ChatBot.cs:228-245` `DisposeAsync` 注释）:
```
// 在WPF中awaiter的实现是推送给UI线程（主线程）执行任务，而不是线程池。
// 这导致很多继体需要在同一个线程中处理。如果其中一个卡死，那后面就无法执行，因此存在死锁的风险。
// 比如XmlFunctionCall在每次发消息都会申请锁，因此IsChatting始终为true，结果为ture时该代码成了死循环，卡死了主线程
// 结果XmlFunctionCall用于释放的续体正好也被推送到主线程的队尾，然而他永远等不到释放的机会。
// 互相等待对方，但永远等不到，于是构成了死锁。
// 利用 Task.Yield，则可以释放线程，将自己重新插入到队尾，于是 Xml 就可以释放他的信号量了。
await Task.Yield();
```

**diva 用 tokio 重新设计**:
- ✅ tokio `select!` 宏天然并发
- ✅ `tokio::spawn` 隔离执行
- ✅ `tokio::sync::Semaphore` + `tokio::task` 跨线程
- ❌ 避免单线程 context（除非显式 `LocalSet`）

### 11.2 DeveloperService AI 自我升级

**Alife 让 AI 写自身 crate 代码, 风险**:
- 编译失败 → Activity 卡死
- 运行时崩溃 → 需要用户手动恢复
- 安全漏洞注入 → 后门

**diva DECISION-v2 §9 "警惕项" 明确**:
> "alife 自我升级(DeveloperService): 风险极大, Laputa 提案流程比这稳得多; 不要让 diva Harness 给 agent 写自身 crate 的代码"

### 11.3 XmlFunctionCaller XML 反射

- 自闭合 XML 标签函数调用（`<StudySkill name="..."/>`）
- 无类型检查, 运行时类型转换
- diva **走 OpenAI tool calling**（type-safe JSON Schema）**已经超过**, 不应回头用 XML

### 11.4 ConfigurationSystem 反射式注入

- `IConfigurable<T>` 反射找接口 → JObject 序列化 → 默认 ctor
- 类型不匹配抛异常
- diva 应该用 serde derive + 显式字段（更安全, 更明确）

---

## 12. 一句话哲学（再强调）

> **Alife 把 agent 当"模块装配体"对待** — Module 三段式 + Autofac DI + Roslyn 热编译. **最强项是 Module 生命周期和 DI 装配** (教科书式, diva 必须借鉴), **最致命的是死锁风险和自我升级** (反面教材, diva 必须避免). 在 diva Harness v2 的 8 P0 + 25 任务里, **alife 贡献的是 #5.1 Module 生命周期 / Autofac DI 等价 + #5.8 Poke 8 事件链 + ConfigurationSystem 借鉴**, 其余用 hermes + 自研.

---

## 附录 A: 关键文件索引

### Alife 核心源码（本次深读）

| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `Alife.Framework/Models/ChatBot.cs` | 8 事件 + Semaphore 互斥 + 流式 + Poke 队列 | 全文 308 |
| `Alife.Framework/Models/Module/InteractiveModule.cs` | Awake/Start/Destroy 三段式 + ITimeIterative | 全文 103 |
| `Alife.Framework/Systems/ModuleSystem.cs` | DI 反射 + Roslyn 热编译 + AssemblyLoadContext + ReloadModules | 全文 356 |
| `Alife.Framework/Systems/ConfigurationSystem.cs` | IConfigurable<T> + storageSystem 自动注入 | 全文 65 |
| `Alife.Function.Memory/MemoryManager.cs` | MemoryMeta 多 Level + 链式压缩 | 全文 251 |
| `Alife.Function.Skill/SkillService.cs` | StudySkill 渐进式披露 + Awake 注册 XmlHandler | 全文 71 |

### Alife 辅助源码（次要, 之前研究已覆盖）

| 文件 | 关键内容 |
|------|---------|
| `Alife.Function.SystemEvent/SystemEventService.cs` | 6 阶段自主报点 + EWake/EWait + UpdatePrompt |
| `Alife.Function.Developer/DeveloperService.cs` | AI 主动写代码 + Roslyn 重编译 + 重启 Activity |
| `Alife.Function.Memory/MemoryStorage.cs` | DuckDB + 向量 + JSON 倒排 |
| `Alife.Function.Memory/TextVectorizer.cs` | 嵌入模型抽象（MiniCPM/Qwen/OpenAI） |
| `Alife.Function.Memory/HistoryCompressor.cs` | 历史压缩接口（实现未读） |

### diva 对照文件

| 维度 | diva 文件 |
|------|-----------|
| Agent Loop | `agent-diva-agent/src/agent_loop/loop_turn.rs` |
| Module 生命周期 | ❌ 待建 (DECISION-v2 §5.1 P0) |
| Autofac DI 等价 | ❌ 待建 (DECISION-v2 §5.1 P0, 用 `inventory` crate) |
| ChatBot 互斥 | ❌ 待建 (用 `tokio::sync::Semaphore`, 防死锁) |
| Poke 8 事件 | ❌ 待建 (DECISION-v2 §5.8 P0) |
| 记忆多 Level | ❌ diva 只有 L0/L1/L2 flat |
| Skill 渐进式披露 | ⚠️ diva `agent-diva-agent/src/skills.rs` 部分实接 |
| 多通道 | ✅ `agent-diva-channels/` 16 个 channel |
| 工具系统 | ✅ `agent-diva-tooling/` Rust trait-based |

---

## 附录 B: 复盘触发器

- [ ] DECISION-v2 §5.1 Module 生命周期实接时, 照搬 Alife `InteractiveModule` 三段式 trait + `IConfigurable<T>` 注入模式 (用 `inventory` crate 替代 Roslyn 编译开销)
- [ ] DECISION-v2 §5.8 Poke 8 事件链全拆时, 借鉴 Alife 8 事件命名 (PokeSend/ChatSend/ChatSent/ChatReceived/ReasoningReceived/ChatOver/ChatHistoryAdd/TokenUsed)
- [ ] DECISION-v2 §5.5 行为审计实接时, 借鉴 Alife §5.3 TokenUsed + ChatHistoryAdd 事件 → `AuditEvent::TokenUsed` + `AuditEvent::ChatHistoryAdd`
- [ ] diva ChatBot 互斥实现时, **必须用 `tokio::sync::Semaphore` + 跨线程 spawn** 避免 Alife WPF 主线程死锁 (绝对不照搬)
- [ ] diva 引入任何 DeveloperService 变体前, **必须走 Laputa 提案流程** (DECISION-v2 §9 警惕项, 严禁 AI 写自身 crate)
- [ ] diva 引入任何 XML 反射式函数调用前, **保持 OpenAI tool calling 类型安全** (diva 已超过 Alife, 不要回头)
- [ ] diva MemoryManager 多 Level 链式压缩实接时, 借鉴 Alife `MemoryMeta(Level, StartTime, EndTime)` 数据结构
- [ ] diva 引入 ConfigurationSystem `IConfigurable<T>` 注入模式时, 用 serde derive + 显式字段替代 JObject 反射 (更安全)
- [ ] alife 上游新版本发布时, 重新跑一遍本研究方法 (核心 6 文件 + 辅助 5 文件), 更新本研究 + DECISION-v2

---

> **报告结束**. 如需进一步细化某个能力 (例如把 Alife `ModuleSystem.Roslyn 热编译` 的具体编译流程写到 diva 设计草稿), 或对比 hermes §2.2.4 Tool Search 渐进披露 + alife §4.1 SkillService.StudySkill 哪个更适合 diva 借鉴, 请指示.
