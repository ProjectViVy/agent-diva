# Alife Plugin/Module 抽象与 DI 装配机制 —— 对 diva-pro 的借鉴建议

> 调研对象:`/Users/mastwet/Desktop/morediva/.workspace/alife/sources/Alife/Alife.Framework/` 及 `Alife.Function/`
> 调研人:大湿代笔(Hermes Agent)
> 日期:2026-06-18
> 对应 DECISION.md 条目:#11(插件系统) + #10(自我升级/热重载)

---

## 0. 结论先行(给设计负责人 TL;DR)

1. **alife 的"二元抽象"实质上是单层抽象**,并非字面意义的 Plugin/Module 双层。
   - `AssemblyLoadContext`(隔离的 DLL 容器)是 "Plugin" 的物理边界
   - `[Module]` 标注的类 + `InteractiveModule<T>` 基类是 "Module" 的逻辑边界
   - "Plugin" 在代码里其实就是 `ModuleLoadContext`(见 `ModuleSystem.cs:15`)
2. **DI 用的是 .NET 原生构造函数注入**(primary constructor),完全靠 `IServiceProvider`,没有第三方容器。模块的依赖在 `AwakeContext.Services` 里取。
3. **diva-pro 不需要完整照抄这套**;最低成本借鉴 3 件东西:
   - `ModuleAttribute` 风格的属性标注 + 自动注册表(`registry.rs` 加宏即可)
   - `ISystemEvent`(三段生命周期:Awake / Start / Destroy)对齐 diva-pro 的 startup/shutdown 钩子
   - 构造函数 DI 的"显式列出依赖"风格,与 diva-pro 现有 trait-based Tool 抽象可叠加

---

## 1. Module 抽象的精确实现

### 1.1 模块基类(无 DI 版本)

文件:`sources/Alife/Alife.Framework/Models/Module/InteractiveModule.cs:9-65`

```csharp
public abstract class InteractiveModule : ISystemEvent
{
    protected Character Character { get; private set; } = null!;
    protected ChatActivity ChatActivity { get; private set; } = null!;
    protected ChatBot ChatBot { get; private set; } = null!;
    protected ChatHistory ChatHistory { get; private set; } = null!;

    public virtual Task AwakeAsync(AwakeContext context) { ... }  // 初始化
    public virtual Task StartAsync(Kernel kernel, ChatActivity chatActivity) { ... }  // AI 激活
    public virtual Task DestroyAsync() { ... }  // 关闭
}
```

**关键观察:**
- 基类只持有 4 个"上下文引用",**不持有任何 system service**
- `ISystemEvent` 接口定义在 `Models/ModuleExtension/ISystemEvent.cs:17-38`,三方法全部 `virtual` 且默认空实现
- 不强制继承 `InteractiveModule`,可以直接 `: ISystemEvent`(给"非交互型"模块用,如纯监控)

### 1.2 带类型标签的派生类 + DI 注入

文件:`sources/Alife/Alife.Function/Alife.Function.SystemEvent/SystemEventService.cs:27-33`

```csharp
[Module("主动事件", "让AI可以获取到各种系统事件的提醒。",
    defaultCategory: "Alife 官方/生活环境",
    LaunchOrder = 100, EditorUI = typeof(SystemEventServiceUI))]
public class SystemEventService(XmlFunctionCaller functionService)
    : InteractiveModule<SystemEventService>, IConfigurable<SystemEventServiceConfig>, ITimeIterative
```

**关键观察:**
- **构造函数即 DI 注入点**——`(XmlFunctionCaller functionService)` 由 DI 容器自动装配
- 通过实现多个 "标记接口"(`IConfigurable<T>`、`ITimeIterative`)来声明能力,而不是继承层级
- `InteractiveModule<T>` 用泛型 `T` 给自己加类型标签,所有辅助方法(`Poke`/`Chat`/`Prompt`)会自动用 `typeof(T).Name` 拼前缀

### 1.3 模块属性(标注 + 元数据)

文件:`sources/Alife/Alife.Framework/Models/Module/ModuleAttribute.cs:5-20`

```csharp
public class ModuleAttribute(
    string name, string description,
    string? url = null, Type? editorUI = null,
    int launchOrder = 0, string defaultCategory = "")
    : Attribute
{
    public string Name { get; private set; } = name;
    public string? Url { get; private set; } = url;
    public Type? EditorUI { get; set; } = editorUI;       // 编辑器 UI 类的 Type 引用
    public int LaunchOrder { get; set; } = launchOrder;   // 启动顺序,负数先跑
    public string DefaultCategory { get; private set; } = defaultCategory;  // 文件夹分类
}
```

**关键观察:**
- `EditorUI = typeof(XxxUI)` 直接用 `Type` 引用另一个类(Blazor 组件)
- `LaunchOrder` 支持负数(`XmlFunctionCaller` 用 `-1000` 抢占最早期)

---

## 2. DI 容器怎么工作

### 2.1 DI 入口:`AwakeContext`

文件:`sources/Alife/Alife.Framework/Models/ModuleExtension/ISystemEvent.cs:9-15`

```csharp
public struct AwakeContext
{
    public Character Character { get; init; }
    public IServiceProvider Services { get; init; }      // 整个 .NET DI 容器
    public IKernelBuilder KernelBuilder { get; init; }   // SemanticKernel 构建器
    public ChatHistoryAgentThread ContextBuilder { get; init; }
}
```

**关键观察:**
- 模块想拿服务 → `context.Services.GetService<T>()`,或者直接在构造函数声明参数
- alife 没自己写容器,直接用 `Microsoft.Extensions.DependencyInjection`

### 2.2 实际模块的"多依赖注入"

文件:`sources/Alife/Alife.Function/Alife.Function.Developer/DeveloperService.cs:13-21`

```csharp
[Module("开发者模式", ...)]
public class DeveloperService(
    CharacterSystem characterSystem,        // 4 个 system 都注入
    ChatActivitySystem chatActivitySystem,
    ModuleSystem moduleSystem,
    XmlFunctionCaller functionCaller) :
    InteractiveModule<DeveloperService>
```

**关键观察:**
- 4 个 system 服务**全靠构造函数签名自动装配**,框架看到参数类型就去容器里找
- 这种"构造签名 = 依赖声明"的模式 = Rust 里非常熟悉的"struct 字段 = 依赖"思路,移植成本极低

---

## 3. Module 注册 / 发现 / 装载生命周期

文件:`sources/Alife/Alife.Framework/Systems/ModuleSystem.cs`

### 3.1 物理隔离层:Collectible AssemblyLoadContext

```csharp
// ModuleSystem.cs:15-25
public class ModuleLoadContext(string[] managedDirectories, string[] unmanagedDirectories)
    : AssemblyLoadContext("ModuleContext", isCollectible: true)
{
    public Assembly LoadDll(string dllPath)
    {
        using var assemblyStream = new MemoryStream(File.ReadAllBytes(dllPath));
        ...
        Assembly assembly = LoadFromStream(assemblyStream, pdbStream);
        return assembly;
    }
}
```

`isCollectible: true` 是关键——表示这个 DLL 容器**可以被卸载回收**(热重载的前提)。

### 3.2 双源加载:DLL + 运行时 C# 源码

```csharp
// ModuleSystem.cs:144-204 (CompileModule)
{
    // 1. 加载已编译的 DLL(用户提前 build)
    foreach (string file in Directory.GetFiles(source, "*.dll", SearchOption.AllDirectories))
        compilingContext.LoadDll(file);

    // 2. 热编译 .cs 源码(用户改了立即生效)
    var syntaxTrees = Directory.GetFiles(source, "*.cs", SearchOption.AllDirectories)
        .Select(file => CSharpSyntaxTree.ParseText(File.ReadAllText(file), ...))
        .ToList();

    var compilation = CSharpCompilation.Create(
        "Modules", syntaxTrees, references,
        new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary)
            .WithOptimizationLevel(OptimizationLevel.Release));

    compilation.Emit(dllPath, pdbPath);
    compilingContext.LoadDll(dllPath);
}
```

**关键观察:**
- **.NET 独有的福利**:Roslyn 编译器内置,运行时就能把 `.cs` 编成 DLL 装入新 context
- 热重载 = `ReloadModules()` → `compilingContext.Unload()` → 重新 `CompileModule()` → `ReloadContext()`
- 全部步骤见 `ModuleSystem.cs:100-108` + `:288-324`

### 3.3 类型扫描:反射 + 属性过滤

```csharp
// ModuleSystem.cs:310-321
foreach (Type type in types)
{
    if (type.GetCustomAttribute<ModuleAttribute>() == null) continue;
    if (type.IsAbstract) continue;
    if (type.IsInterface) continue;
    moduleTypes.Add(GetModuleID(type), type);  // ID = type.FullName
}
```

**关键观察:**
- 注册表 `Dictionary<string, Type>` —— Key 是 `Type.FullName`
- 无外部注册代码,**纯靠反射扫 `[Module]` 标注**

### 3.4 启动顺序 + 文件夹分类

`ModuleSystem.cs:325-355` 的 `SyncFolder()`:
- 按 `defaultCategory`(`"Alife 官方/生活环境"`)自动建文件夹树
- `LaunchOrder` 控制 `AwakeAsync` 调用的先后

---

## 4. 与 diva-pro 的对应关系

### 4.1 当前 diva-pro 的"事实 Plugin"层级

| alife 概念     | diva-pro 现状                          | 性质 |
|----------------|----------------------------------------|------|
| Framework      | `agent-diva-core` + `agent-diva-tooling` | 静态编译的"框架本体" |
| Plugin(物理)   | `agent-diva-tools` / `agent-diva-providers` / `agent-diva-channels` | **编译期 plugin** —— Cargo workspace member |
| Module(逻辑)   | `Tool` trait + `ToolRegistry`(`agent-diva-tooling/src/{base,registry}.rs`)| 运行时注册单元 |

**结论**:diva-pro 当前是**单层 Plugin 抽象**,通过 workspace member 做物理边界,通过 `Tool` trait + `Arc<dyn Tool>` 做逻辑边界。
没有运行时热加载(全部编译期)。

### 4.2 引入 Module 抽象需要改的 crate

| 改动点                          | 涉及 crate                          | 优先级 |
|--------------------------------|-------------------------------------|--------|
| 新增 `agent-diva-module` crate(承载 `Module` trait + `ModuleRegistry`) | 新 crate | **P0** |
| 给 `Tool` 加 `launch_order` + `category` 字段 | `agent-diva-tooling` | P1 |
| 三段生命周期钩子 `awake/start/destroy`     | `agent-diva-core`(或新 module crate) | P1 |
| 反射 / 宏扫 `#[derive(Module)]` 注册       | `agent-diva-module`                  | P0 |
| 热重载(运行时编译)**不建议**              | ——                                  | P3,见 §5.3 |

---

## 5. 对 diva-pro 的 actionable 建议

### 5.1 ✅ P0(必做):定义 `Module` trait,借鉴构造签名风格

参考 `InteractiveModule<T>` + `ModuleAttribute`,在 `agent-diva-module` 新 crate 里写:

```rust
// 伪代码 / 示意,具体 API 待 diva-pro 风格化
pub trait Module: Send + Sync {
    fn name(&self) -> &str;
    fn launch_order(&self) -> i32 { 0 }
    fn category(&self) -> &str { "默认" }
}

#[async_trait]
pub trait SystemEvent: Module {
    async fn awake(&mut self, ctx: &mut AwakeContext<'_>) -> Result<()>;
    async fn start(&mut self) -> Result<()> { Ok(()) }
    async fn destroy(&mut self) -> Result<()> { Ok(()) }
}

pub struct AwakeContext<'a> {
    pub bus: &'a dyn EventBus,
    pub services: &'a dyn ServiceProvider,  // 或直接 Arc<dyn Any>
    pub logger: &'a TracingLogger,
}
```

**借鉴点**:构造签名即依赖声明——diva-pro 用 `Arc<dyn ToolRegistry>` 这种"显式注入"比 .NET 反射扫构造函数参数更 Rust-idiomatic,但**思路一样**。

### 5.2 ✅ P1(可做):给 Tool 加 launch_order + category 元数据

`agent-diva-tooling/src/base.rs` 的 `Tool` trait 目前只有 `name`/`description`/`parameters`/`execute`,缺两样:
- `launch_order: i32`(默认 0)
- `category: &str`(默认 "工具")

加好后 `ToolRegistry` 自动支持 `register_with_order()`,然后**所有 `agent-diva-tools/*` 业务模块都不改代码就能享受到启动排序**(因为现有实现都已是 `Arc<dyn Tool>`,加字段不影响)。

### 5.3 ❌ 不建议做的事:Roslyn 式运行时热编译

alife 的 `CompileModule()` 用了 `Microsoft.CodeAnalysis.CSharp` 把 `.cs` 编进新 `AssemblyLoadContext`。**Rust 没有等价物**:
- `rustc` 不能作为库被调来做 JIT(只有 nightly 的 `rustc_private` 勉强,但 ABI 不稳定)
- `wasmtime` + Rust → wasm 路线理论可行,但 5-10 倍启动延迟、缺 SIMD、缺 std
- **结论**:diva-pro 热重载走"skill 文件热重载"(DECISION.md 已写),**模块代码热重载 = 不做**

### 5.4 ✅ P1(可做):把 `ModuleAttribute` 的"Type 引用 EditorUI"改成"trait object 引用"

alife 的 `EditorUI = typeof(SystemEventServiceUI)` 是 Blazor Component 类型。
diva-pro 对应物是 **`ModuleUi: dyn ModuleUiRender` trait**,让每个模块声明自己的 Tauri/Web UI 渲染器。
借鉴价值:**模块自带 UI** 是个值得保留的好设计(用户不用查文档就知道这个模块怎么配)。

### 5.5 ⚠️ 待验证(给后续调研者)

1. alife 的 `ModuleSystem.SetExtraContext()` 用途(`ModuleSystem.cs:216-220`)—— 看起来是给 plugin 提供额外的 dll 搜索路径,但没有调用方,可能是预留 API
2. `defaultAssemblies` 字段(`ModuleSystem.cs:239`)—— 防止重复加载宿主 DLL 的清单,diva-pro 借鉴时需不需要这层?
3. `ModuleLoadContext` 的 RID 处理(`ModuleSystem.cs:68-72`)—— diva-pro 跨平台 binary 兼容目前走"workspace 编译",是否需要"运行时按平台搜 .so/.dll"?

---

## 6. 一句话总结

> alife 的 "Plugin vs Module" 其实只是**物理 DLL 隔离 vs 逻辑 [Module] 标注**的一对儿;真正的精髓是 **"构造签名即依赖 + 三段生命周期 + 元数据驱动的反射注册"**。diva-pro 借鉴前两个零成本,第三个(反射注册)Rust 里用过程宏替代。

**借鉴优先级排序**:`Module` trait + DI(P0) > Tool 元数据扩展(P1) > 模块自带 UI(P1) > 热编译热重载(不做)

---

## 附录:核心文件路径速查

| 角色               | 文件路径(alife 内)                                                              |
|-------------------|---------------------------------------------------------------------------------|
| Module 基类       | `sources/Alife/Alife.Framework/Models/Module/InteractiveModule.cs`              |
| Module 标注       | `sources/Alife/Alife.Framework/Models/Module/ModuleAttribute.cs`                |
| 三段生命周期接口  | `sources/Alife/Alife.Framework/Models/ModuleExtension/ISystemEvent.cs`          |
| 时序更新接口      | `sources/Alife/Alife.Framework/Models/ModuleExtension/ITimeIterative.cs`        |
| 配置接口          | `sources/Alife/Alife.Framework/Models/ModuleExtension/IConfigurable.cs`         |
| 注册/扫描/装载    | `sources/Alife/Alife.Framework/Systems/ModuleSystem.cs`                         |
| 物理 DLL 隔离     | `sources/Alife/Alife.Framework/Systems/ModuleSystem.cs:15-73` (`ModuleLoadContext`) |
| UI 基类           | `sources/Alife/Alife.Framework/Models/Module/ModuleUIBase.cs`                    |
| 实际示例          | `sources/Alife/Alife.Function/Alife.Function.SystemEvent/SystemEventService.cs` |
| 多依赖示例        | `sources/Alife/Alife.Function/Alife.Function.Developer/DeveloperService.cs`     |