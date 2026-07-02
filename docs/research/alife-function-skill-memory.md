# alife 三大模块精读 → diva-pro 借鉴建议

> 子代理 SA2 产出 · 2026-06-18

## A) `XmlFunctionCaller` —— 标签流式函数调用框架

**核心数据结构**:`XmlHandlerTable` + `XmlHandler`(Method 反射到 C# 对象);`DocumentMode` 决定是否进系统提示词。

**关键代码 1**(显式/隐式分桶 + Trigger 懒加载):
```csharp
public void RegisterHandler(XmlHandler handler, DocumentMode documentMode) {
    handlerTable.Register(handler);
    if (documentMode == DocumentMode.Implicit) {
        implicitHandlers.Add(handler);
        AddImplicitTrigger(handler);   // 注册 foo_Trigger 一行提示
    } else if (documentMode == DocumentMode.Explicit)
        explicitHandlers.Add(handler); // 完整文档注入 system prompt
}
```

**关键代码 2**(AI 调用 `<foo_Trigger/>` 时才推送完整文档):
```csharp
void AddImplicitTrigger(XmlHandler source) {
    var xmlHandler = new() { Name = source.Name + "_Trigger" };
    xmlHandler.Functions.Add(new XmlFunction {
        Name = source.Name!.ToLower(),
        Invoker = (ctx, t) => { Poke(GetExplicitDocument(source)); return Task.CompletedTask; }
    });
}
```
流式:`ChatReceived → executor.Feed()`、`ChatSent → Flush + WaitToInactive`,按 `minBreakingLength=9` 在标点处切段执行。

**diva-pro 对应路径**:`agent-diva-providers/`(LLM 抽象)+ `agent-diva-tools/`(工具注册表)。diva-pro 已用 JSON schema 走 OpenAI function calling,**没有显隐分桶**,所有 tool 完整描述每次都进 system prompt。alife 价值在「**Implicit 触发模式**」——把 N 个工具压成一行入口,需要时 AI 自取。

## B) `SkillService` —— 按需读手册的技能框架

整个模块就 71 行,极简。

**关键代码**(注册一个隐式 StudySkill 函数,Explanation 里只列名字):
```csharp
public void StudySkill(string name) {
    string skillDocPath = Path.Combine(skillsPath, name, "SKILL.md");
    string skillDoc = File.ReadAllText(skillDocPath);
    string[] appendFiles = Directory.GetFiles(...);
    Poke($""" [{nameof(StudySkill)}] 已读取 {name} skill ... {skillDoc} """);
}
// AwakeAsync: 把所有 skill 名拼到 Explanation,RegisterHandler(..., DocumentMode.Implicit)
```
存储 = `{StorageFolder}/Skills/<name>/SKILL.md` + 同目录附属文件。

**diva-pro 对应路径**:`agent-diva-agent/src/skills.rs`(已有 `SkillsLoader` 解析 SKILL.md frontmatter,有 `always: bool` 字段)。关键差异:
- diva-pro 把 `always=false` 的 skill **不注入** system prompt,AI 根本看不到;
- alife 把 `always=false` 的 skill 名列在 StudySkill 的 Explanation 里,AI 主动调 `StudySkill(name)` 加载。

→ diva-pro 缺的就是这个「**列出但按需读**」的中间档。

## C) `MemoryService` + `MemoryManager` —— 层级压缩 + 嵌套回想

**核心数据**:`MemoryMeta(Level, StartTime, EndTime)`,`Name = "{Level}-{yyyyMMddHHmmss}-{yyyyMMddHHmmss}"`(例 `2-20260421014905-20260512022747`)。**Level 越大压缩次数越多**。

**关键代码 1**(`Filter`:每次 AI 说完后扫描聊天历史,凑够阈值就升一档):
```csharp
if (areaCount >= areaCompressionThreshold && areaLevel + 1 <= maxCompressionLevel) {
    string range = $"...{areaLevel}级, 从 {startTime} 到 {endTime} ...";
    string? summary = await compressor.Compress(chatHistory, range);
    await SaveMemory(areaLevel + 1, ..., summary, fullContent, chatHistory, areaStart);
    chatHistory.RemoveRange(areaStart + 1, areaCompressionCount);
}
```
阈值:L0 = 100 条才压;L≥1 时只 4 条就压;批量 L0=60 / L≥1=3。原始内容落盘 `{storagePath}/L0/<id>.txt`。

**关键代码 2**(`Recall` 嵌套解压,语义链):
```
L3 摘要 → Recall → L2 摘要 → Recall → L1 摘要 → Recall → 原始对话
```
`Search` 走 `TextVectorizer`(内存向量),按 `keyword+level+time` 过滤;`CompressPrompt` 有 8 条质量规则(去冗余、保留画像、压缩后用具体人名),`Probability=0.4` 随机触发避免每次都花 LLM 钱。

**diva-pro 对应路径**:`agent-diva-core/src/memory/`(`MemoryManager` + `storage.rs` 操作 `MEMORY.md` / `HISTORY.md`)+ `agent-diva-laputa/src/memory_provider.rs`(Laputa 迁移)+ `agent-diva-laputa/src/proposals.rs`。diva-pro 是**扁平 Markdown + 事件流**,没有层级压缩、没有递归回想;Laputa 是把不同来源 memory 提案/合并,**不是压缩**。

## 借鉴建议 —— 哪个先做

**建议排序**:**Skill 按需加载 > 工具 on-demand trigger > 记忆层级(最后做,谨慎)**

1. **先做 Skill 按需加载**(1-2 天 PoC)
   diva-pro 已有 `SkillsLoader`,只缺一个 `agent-diva-tools` 内置的 `study_skill(name: String)` 工具,server 端读 `workspace/skills/<name>/SKILL.md` 推回对话上下文。同时 `SkillsLoader.list_skills()` 在 system prompt 渲染时区分 `always` / `on_demand`:always 注入全文,on_demand 只列 `name + description`。改 1 个工具 + 1 个 renderer 分支,零破坏。

2. **再做工具 on-demand trigger**(中等,看工具数量)
   把 `agent-diva-tools` 的 `ToolSpec` 加 `pub enum DocVisibility { Always, OnDemand { trigger_name: String } }`;渲染 system prompt 时只渲染 trigger 提示行 + 一个 `get_tool_doc(name)` 内置工具,AI 调用时由 providers 层把完整 schema 注入下一轮。价值在工具数 > 30 时凸显,目前工具少,优先级低于 1。

3. **最后做记忆层级**(大坑,先设计 spike)
   diva-pro 的 `MEMORY.md` / `HISTORY.md` 写法和 alife 的 `Level-Time-Time` 命名 + L0..L7 目录布局**心智模型完全不同**;且 diva-pro 已有 Laputa 提案/合并语义,alife 的 `Filter` 内联压缩会绕过 Laputa。建议**先写一个 design doc 对照**:`MemoryManager.Filter` ↔ `LaputaProposal`、`MemoryStorage.L0..L7` ↔ `memtle` 后端,再决定是改造 manager.rs 还是新增 `agent-diva-laputa/hierarchical.rs`。**待验证**:diva-pro 是否已在 Laputa 里有摘要/迁移雏形,需另起一轮调研。

**最不该先做**:把 alife 的 `XmlHandler` 整套移植过来。diva-pro 已用 OpenAI 兼容 JSON tool call,XML 流式 + `minBreakingLength=9` 的方案是 SK 时代的产物,迁过去等于重写 providers,得不偿失——只取其「显隐分桶 + on-demand doc」思想即可。
