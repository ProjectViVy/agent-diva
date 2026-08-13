# alife Harness 能力 vs diva 现状 —— 决策前清单

> 提取时间: 2026-06-19
> 来源: `harness-engineering-three-way-detailed-checklist.md` (22 维度,324 行)
> 目的: 列出 "alife 有 / diva 缺" 的所有 harness 能力,作为下一步决策输入

---

## 总览

**alife 有 / diva 缺 = 14 项**(跨 8 个维度)

按影响面分:
- **架构级**(4 项):影响整个 agent runtime 设计
- **模块级**(5 项):影响单个 Module/Plugin 抽象
- **能力级**(5 项):具体工具/feature

---

## 14 项 gap 清单(按维度分组)

### 1. Streaming / Async 模型(2 项)

| 子项 | diva 现状 | alife 实现 |
|---|---|---|
| **取消机制** | ❌ 不显式(`tokio::select!` 局部) | ✅ `CancellationToken` 注入 `StartAsync` |
| **流式 UI 渲染** | ❌ 不支持(CLI 静态输出) | 🟡 `Alife.DeskPet` Razor 组件可异步更新 |

### 2. Toolset 组织(1 项)

| 子项 | diva 现状 | alife 实现 |
|---|---|---|
| **MCP 集成** | 🟡 schema 里有(`MCPServerConfig`),未实现完整生命周期 | ✅ `Alife.Function.Mcp` 项目存在,生命周期完整 |

### 3. Skill 系统(2 项)

| 子项 | diva 现状 | alife 实现 |
|---|---|---|
| **附随文件加载**(resources/scripts) | 🟡 `*` 全目录读 | ✅ `appendFiles` 列表 + python 执行,精细控制 |
| **Skill install 多入口** | ❌ 不支持 | ✅ `modelscope skills add` CLI 命令 |

### 4. Memory 架构(1 项)

| 子项 | diva 现状 | alife 实现 |
|---|---|---|
| **分层/分级记忆 (level)** | ❌ 单层(MEMORY.md + HISTORY.md) | ✅ `MemoryMeta(Level, StartTime, EndTime)` 多级(Level 0-N),`compressionThreshold` per-level |

### 5. 多 session / 并发(3 项)

| 子项 | diva 现状 | alife 实现 |
|---|---|---|
| **多用户隔离** | ❌ 单用户(无 user_id 概念) | 🟡 `ChatActivity` 角色级独立 |
| **多 session 隔离** | 🟡 session/manager.rs 有 cache + disk | ✅ `ChatActivity` per-channel 完全隔离 |
| **并发 session 互斥** | ❌ 无显式 | ✅ `ChatBot.RequestChatAsync` Semaphore(0/1) |

### 6. Tool 硬约束(模块加载)(2 项)

| 子项 | diva 现状 | alife 实现 |
|---|---|---|
| **动态加载(按需启用)** | ❌ 不支持(全部加载) | ✅ 模块懒加载(CLR 反射) |
| **热重载 (hot reload)** | ❌ 不支持(无 file watcher) | 🟡 `ModuleSystem.ReloadModules` 是模块热重载(非配置) |

### 7. 持久化(1 项)

| 子项 | diva 现状 | alife 实现 |
|---|---|---|
| **备份/恢复 (corrupt)** | ❌ 无 | 🟡 `StorageSystem` 通用持久化 |

### 8. 模块抽象(2 项)

| 子项 | diva 现状 | alife 实现 |
|---|---|---|
| **Skill guard (install 拦截)** | 🟡 Laputa proposal 流程 | 🟡 `ReloadModules` 流程 |
| **同能力多入口对位** | 🟡 CLI/GUI/IM 共用 core bus | 🟡 DeskPet 与 ChatBot 共用 Framework |

---

## 我的分类(供决策)

### A 组 —— 必须做(架构级,影响大)

1. **多用户隔离 + 多 session 隔离 + 并发互斥**(3 项合一)
   - 当前:单用户,无隔离 → diva 多用户时必崩
   - alife 路径:per-channel ChatActivity + Semaphore(0/1)
   - 工作量:2-3 周

2. **分层/分级记忆 (Level 0-N)**
   - 当前:单层 MEMORY.md/HISTORY.md,无压缩分级
   - alife 路径:`MemoryMeta(Level, StartTime, EndTime)` + `compressionThreshold` per-level
   - 工作量:3-4 周(含 DuckDB 或 SQLite 向量集成)

3. **取消机制(CancellationToken 全链路)**
   - 当前:`tokio::select!` 局部,无统一 token
   - alife 路径:`CancellationToken` 注入到每个 `StartAsync`
   - 工作量:1 周

### B 组 —— 应该做(能力级,补足 harness)

4. **动态加载(模块懒加载)**
   - 当前:全量加载,启动慢
   - alife 路径:CLR 反射懒加载
   - diva 路径:Rust 编译期模块 + feature flag
   - 工作量:1 周

5. **MCP 完整生命周期**
   - 当前:schema 有,行为缺
   - alife 路径:`Alife.Function.Mcp` 完整 client
   - 工作量:1-2 周

6. **附随文件加载精细控制**
   - 当前:`*` 全目录读(浪费 token)
   - alife 路径:`appendFiles` 列表精确控制
   - 工作量:0.5 周

### C 组 —— 看情况(产品决策)

7. **Skill install CLI**
   - 当前:无
   - alife 路径:`modelscope skills add` CLI
   - 工作量:0.5 周

8. **热重载(模块)**
   - 当前:无
   - alife 路径:`ModuleSystem.ReloadModules`(Roslyn 编译 + ALC 卸载)
   - diva 路径:WASM 插件 + wasmtime 实例(复杂度高)
   - 工作量:3-4 周(POC),延后到 v2+

9. **流式 UI 渲染**
   - 当前:CLI 静态
   - alife 路径:DeskPet Razor
   - diva 路径:Tauri v2 reactive UI(已有)
   - 工作量:1 周(主要补 CLI streaming)

### D 组 —— 不建议做(alife 弱 / diva 已有更好的)

10. **Skill guard install** —— diva 已有 Laputa proposal 流程,更强,跳过
11. **同能力多入口对位** —— diva core bus 已对位,无需再补
12. **备份/恢复** —— Laputa changelog + atomic write 部分覆盖,完整备份是 v2 议题

---

## 决策建议

**核心提议**: A 组 3 项必做,B 组前 2 项高 ROI。

合计 ~10-12 周 1 人,正好对应 Harness Engineering 1 个 wave 的核心工作量。

---

## 待你拍板的输入问题

1. A 组 3 项:全做 / 选做 / 推迟?
2. B 组前 2 项(动态加载 + MCP 完整生命周期):都做?
3. C 组:挑哪个?(尤其 #8 热重载要不要做 POC)
4. D 组:确认跳过?
