# Plugin Architecture Reassessment

本文重新评估 `agent-diva` 的插件实施方式。结论不是“先做 channel plugins”，而是：

- `agent-diva` 应直接设计成 **通用插件框架**
- `channel` 只是第一批落地能力之一
- 初期优先实现的插件能力面建议为：
  - `channel`
  - `provider`
  - `tool`
  - `service`
- 后续再扩展到：
  - `memory`
  - `context_engine`
  - `media_understanding`
  - `web_search`
  - `sandbox`
  - `command`

本文只写方案，不写代码。

## 背景

上一轮 nanobot 对标调研中，已确认 `agent-diva` 当前缺少外部 channel plugin 机制。

但如果直接以“channel plugin”作为目标，容易过早把插件边界收窄为：

- 只解决聊天通道接入
- 继续把 provider、tool、service 等扩展点留在核心仓库
- 未来再做第二套、第三套扩展机制

这会让 `agent-diva` 的扩展能力分裂成多个平行体系，不利于长期维护。

因此，本次进一步参考 `.workspace/openclaw`，重评估插件机制应如何落地。

## OpenClaw 的关键观察

## 1. OpenClaw 的插件不是“频道专用机制”

OpenClaw 官方插件文档明确写明，插件可以扩展：

- channels
- model providers
- tools
- skills
- speech
- image generation
- media understanding
- HTTP routes
- CLI commands
- services

这说明 OpenClaw 的设计起点不是“给 channel 开个插件口”，而是先做统一插件注册面，再由不同能力类型挂接进去。

关键证据：

- `.workspace/openclaw/docs/tools/plugin.md`

## 2. 它有统一注册 API，而不是多个平行扩展入口

OpenClaw 的插件入口形态是：

```ts
export default definePluginEntry({
  id: "my-plugin",
  name: "My Plugin",
  register(api) {
    api.registerProvider(...)
    api.registerTool(...)
    api.registerChannel(...)
  },
});
```

文档中列出的注册方法包括：

- `registerProvider`
- `registerChannel`
- `registerTool`
- `registerSpeechProvider`
- `registerMediaUnderstandingProvider`
- `registerImageGenerationProvider`
- `registerWebSearchProvider`
- `registerHttpRoute`
- `registerCommand` / `registerCli`
- `registerContextEngine`
- `registerService`

这意味着插件实现者面对的是一个统一的宿主 API，而不是按能力类型学习多套加载协议。

关键证据：

- `.workspace/openclaw/docs/tools/plugin.md`
- `.workspace/openclaw/src/plugins/registry.ts`
- `.workspace/openclaw/src/plugin-sdk/core.ts`

## 3. 发现机制是 manifest 驱动，而不是硬编码某一类扩展目录

OpenClaw 支持多种插件来源，按顺序发现：

1. `plugins.load.paths`
2. workspace 扩展目录
3. 全局扩展目录
4. bundled plugins

并且 npm 包通过 `package.json` 中的 `openclaw.extensions` 声明入口。

例如：

```json
{
  "openclaw": {
    "extensions": ["./index.ts"]
  }
}
```

这套机制的重点在于：

- 插件来源统一
- 插件包结构统一
- channel / provider / sandbox 等能力不需要各自定义一套发现协议

关键证据：

- `.workspace/openclaw/docs/tools/plugin.md`
- `.workspace/openclaw/src/plugins/discovery.ts`
- `.workspace/openclaw/extensions/openshell/package.json`
- `.workspace/openclaw/extensions/nvidia/package.json`

## 4. 插件注册表按能力分类汇总

OpenClaw 的 `PluginRegistry` 不是单一列表，而是按能力类型分桶：

- `channels`
- `providers`
- `tools`
- `services`
- `commands`
- `httpRoutes`
- `speechProviders`
- `mediaUnderstandingProviders`
- `imageGenerationProviders`
- `webSearchProviders`

这说明：

- “插件”是总概念
- “某类扩展点”是注册表里的一个 capability bucket

这对 `agent-diva` 很重要，因为它允许我们先做一套插件宿主，再逐步开放新的 bucket，而不是每开放一类能力都重做架构。

关键证据：

- `.workspace/openclaw/src/plugins/registry.ts`

## 5. 运行时会区分不同 surface 的注册表视图

OpenClaw 的运行时不仅有全局 active registry，还会针对不同 surface 管理可见插件集，例如：

- `httpRoute`
- `channel`

并支持 pin/release，避免运行时重载时把正在使用的 surface 意外替换掉。

这说明 OpenClaw 在设计上已经考虑了：

- 启动期加载
- 热重载/再加载
- 某些 surface 需要稳定快照

对 `agent-diva` 的启发是：如果将来支持插件刷新，不能只做“全局替换整份 registry”，而要考虑不同运行面上的稳定性。

关键证据：

- `.workspace/openclaw/src/plugins/runtime.ts`

## 6. 插件可以有 registration mode，不同阶段注册不同能力

OpenClaw 的插件 API 支持不同 registration mode。比如 `openshell` 插件在非 `full` 模式下直接跳过。

这意味着插件既可以：

- 参与配置/发现/校验阶段
- 也可以只在完整运行时阶段注册真正的执行能力

这对于 `agent-diva` 很有价值，因为：

- GUI/Manager 可能只需要读插件元数据和配置 schema
- Gateway/Agent runtime 才需要真正启动服务和执行逻辑

关键证据：

- `.workspace/openclaw/extensions/openshell/index.ts`

## 7. OpenClaw 还做了明显的安全边界和 SDK 边界

它对插件做了：

- 发现路径顺序和优先级控制
- 路径逃逸检查
- world-writable / ownership 检查
- plugin SDK import boundary 测试

这说明插件机制不是“能 load 就行”，而是被当成一个带攻击面的系统设计。

关键证据：

- `.workspace/openclaw/src/plugins/discovery.ts`
- `.workspace/openclaw/test/plugin-extension-import-boundary.test.ts`

## Nanobot vs OpenClaw：插件模型的本质差异

这一部分专门回答三个问题：

1. `nanobot` 的插件到底是怎么实现的
2. `openclaw` 的插件到底是怎么实现的
3. 两者的差异到底在哪里

## 1. nanobot 的插件本质上是“外部 channel 扩展”

`nanobot` 的插件实现非常聚焦，只围绕 `channel` 一类能力展开。

它的实现链路是：

1. 扫描内建 `nanobot.channels` 包中的模块名
2. 用 `importlib.metadata.entry_points(group="nanobot.channels")` 加载外部插件
3. 将“内建 channel”和“外部 channel plugin”做合并
4. 由 `ChannelManager` 直接实例化这些 channel
5. `onboard` 时把各 channel 的 `default_config()` 写入配置

这说明 nanobot 的插件模型本质是：

- 扩展点只有一个：`channel`
- 插件只需要暴露一个 `BaseChannel` 子类
- 插件发现和配置注入都直接绑定在 channel 子系统上
- 没有统一插件 manifest
- 没有统一 capability registry
- 没有 provider / tool / service / memory / command 级别的统一插件模型

也就是说，nanobot 的“插件”更准确的说法其实是：

> 外部聊天通道接入机制

它解决的问题是：

- 让第三方在不改 nanobot core 的前提下增加新 channel

它没有解决的问题是：

- 如何统一扩展 provider
- 如何统一扩展 tool
- 如何统一扩展 service
- 如何统一扩展 memory/context engine
- 如何统一管理插件安全、生命周期、诊断、GUI 可见性

关键证据：

- `.workspace/nanobot/nanobot/channels/registry.py`
- `.workspace/nanobot/nanobot/channels/base.py`
- `.workspace/nanobot/nanobot/cli/commands.py`
- `.workspace/nanobot/tests/channels/test_channel_plugins.py`
- `.workspace/nanobot/docs/CHANNEL_PLUGIN_GUIDE.md`

## 2. openclaw 的插件本质上是“统一宿主扩展框架”

`openclaw` 的实现目标完全不同。

它不是先问“怎么让外部加一个 channel”，而是先问：

> 宿主系统应如何统一接纳不同类型的能力扩展？

因此 OpenClaw 的设计是：

- 先定义统一插件入口
- 再定义统一插件 API
- 再定义统一 registry
- 再按 capability bucket 分类注册

OpenClaw 的插件可以扩展的能力明确包括：

- `channel`
- `provider`
- `tool`
- `speech provider`
- `media understanding provider`
- `image generation provider`
- `web search provider`
- `http route`
- `command/cli`
- `service`
- `context engine`
- `memory slot`

在实现上，OpenClaw 的插件具备这些特征：

### 统一入口

插件通过统一入口导出：

```ts
export default definePluginEntry({
  id: "my-plugin",
  name: "My Plugin",
  register(api) {
    api.registerProvider(...)
    api.registerTool(...)
    api.registerChannel(...)
  },
});
```

这意味着：

- channel/provider/tool 不是不同插件系统
- 它们只是同一个插件宿主下的不同注册类别

### 统一发现

插件可来自：

- `plugins.load.paths`
- workspace 扩展目录
- 全局扩展目录
- bundled plugins

并通过包 manifest 中的 `openclaw.extensions` 声明入口。

### 统一注册表

运行时有统一 `PluginRegistry`，内部再分 bucket：

- `channels`
- `providers`
- `tools`
- `services`
- `commands`
- `httpRoutes`
- `speechProviders`
- `mediaUnderstandingProviders`
- `imageGenerationProviders`
- `webSearchProviders`

### 统一安全与边界

OpenClaw 对插件不仅做加载，还做：

- 路径和 ownership 检查
- import boundary 检查
- slot 管理
- registration mode 区分
- runtime surface pinning

这说明 OpenClaw 把“插件”看成平台级架构能力，而不是局部功能。

关键证据：

- `.workspace/openclaw/docs/tools/plugin.md`
- `.workspace/openclaw/src/plugins/discovery.ts`
- `.workspace/openclaw/src/plugins/loader.ts`
- `.workspace/openclaw/src/plugins/runtime.ts`
- `.workspace/openclaw/src/plugins/registry.ts`
- `.workspace/openclaw/src/plugins/types.ts`
- `.workspace/openclaw/src/plugin-sdk/core.ts`

## 3. 两者差异的本质，不是复杂度，而是设计目标

表面上看是：

- nanobot 简单
- openclaw 复杂

但真正的区别不是代码多少，而是它们在解决不同层级的问题。

### nanobot 解决的是：

- 如何在极小核心上增加新聊天通道

因此它采用的是：

- channel-specific extension point

### openclaw 解决的是：

- 如何让整个 AI 平台以统一方式接纳多种类型的外部能力

因此它采用的是：

- host-level extensibility architecture

所以如果只看“插件”这个词，会误以为两者是同一类设计的复杂版与简化版。

实际上不是。

更准确的对照是：

- `nanobot plugin` ≈ 外部 channel adapter 机制
- `openclaw plugin` ≈ 平台级扩展系统

## 4. 对 agent-diva 的直接启发

这也是为什么 `agent-diva` 不应该直接照着 nanobot 的插件实现来做。

如果直接模仿 nanobot，大概率会得到：

- `agent-diva.channels.plugins`
- 若干扫描目录 / manifest 约定
- 专门给 channel 用的加载逻辑

然后在之后又不得不为：

- provider
- tool
- service
- memory
- context engine

再各做一遍新的扩展机制。

这会导致：

- 发现协议重复
- 配置结构重复
- 生命周期管理重复
- GUI/Manager 状态展示重复
- 安全模型重复

所以，正确的借鉴方式应当是：

- **借鉴 nanobot 的节制**
  - 第一阶段不要做过多能力面
  - 先支持最有价值的几类 capability
- **借鉴 openclaw 的抽象方式**
  - 插件是统一宿主能力
  - channel 只是其中一个 bucket

## 对 agent-diva 的最终建议

建议把 `agent-diva` 的插件定义为：

> 一个通过统一 manifest 和宿主 API 向系统注册能力的通用扩展单元。

而不是：

> 一个额外的 channel 模块。

### Phase 1 建议开放的 bucket

- `channel`
- `provider`
- `tool`
- `service`

理由：

- 足够覆盖当前最有价值的扩展面
- 与现有 crate 分层最匹配
- 不会像 openclaw 一样一步扩到非常宽

### Phase 2 再开放

- `memory`
- `context_engine`
- `media_understanding`
- `web_search_provider`
- `sandbox_backend`
- `command`

### 实施方式建议

- 不采用 nanobot 那种 `entry_points(group="...channels")` 的 channel-only 机制
- 也不在 Phase 1 直接走 Rust 动态库插件
- 更适合：
  - 统一 manifest
  - 统一 registry
  - 外部进程宿主
  - bucket 化注册

## 小结

可以用一句话概括：

- `nanobot` 的插件，是“在极简 agent 上开放一个外部 channel 接口”
- `openclaw` 的插件，是“为整个平台建立一套统一扩展宿主”

`agent-diva` 应该选择后者的方向，但在落地节奏上保留前者的克制。

## 对 agent-diva 的核心结论

## 不建议做“只支持 channel 的插件机制”

原因有四个：

### 1. 会把架构边界做窄

如果先定义一套 `ChannelPlugin` 专用加载机制，后续给 provider/tool/service 扩展时，很大概率要：

- 复制加载器
- 复制 manifest 约定
- 复制配置 enable/disable 逻辑
- 复制安全与诊断逻辑

这会让插件系统碎片化。

### 2. 不能解决最有价值的扩展诉求

`agent-diva` 当前最值得插件化的，不只有 channel：

- provider 登录和 provider 目录扩展
- sandbox/runtime backend
- media understanding
- web search provider
- 未来 memory/context engine 这种互斥型能力

如果先把插件限定为 channel，会延缓这些高价值扩展点的抽象统一。

### 3. Rust 动态加载天然更需要统一宿主层

在 Rust 里直接做动态库插件并不理想，ABI 稳定性差，跨版本兼容成本高。

因此更适合先定义一层宿主协议，然后让插件通过：

- 外部进程
- WASM component
- 受控桥接协议

接入统一 registry。

既然需要先做宿主层，就更应该把能力面一次性抽象对，而不是先做 channel-only 方案再返工。

### 4. GUI / Manager / Gateway 都会消费插件元数据

插件不仅影响 Gateway 运行，还会影响：

- GUI 设置页
- Manager 的状态接口
- 配置校验
- 文档和诊断输出

所以插件系统必须从一开始就是“平台能力”，而不是 channel 子系统内部能力。

## 建议的 agent-diva 插件目标模型

## 插件总线，而不是插件特例

建议把插件定义为：

> 一个通过统一 manifest + 宿主 API 向 `agent-diva` 注册能力的扩展单元。

### 第一批能力 bucket

第一阶段建议开放：

- `channel`
- `provider`
- `tool`
- `service`

原因：

- 这四类最接近当前 `agent-diva` 现有 crate 分层
- 价值高
- 用户可感知明显
- 与 OpenClaw 的实践最接近

### 第二批能力 bucket

第二阶段再开放：

- `web_search_provider`
- `media_understanding_provider`
- `speech_provider`
- `sandbox_backend`
- `command`

### 独占 slot 能力

建议从一开始为下列能力预留 slot 机制：

- `memory`
- `context_engine`

原因：

- 这两类能力天然是互斥或主导型，不适合多个插件同时生效
- 未来如果要做高级记忆引擎或上下文引擎，slot 机制会比简单 enable/disable 更稳

## 建议的配置模型

建议参考 OpenClaw，但结合 `agent-diva` 当前 JSON 配置风格，保留如下结构：

```json
{
  "plugins": {
    "enabled": true,
    "allow": [],
    "deny": [],
    "load_paths": [],
    "entries": {
      "my-plugin": {
        "enabled": true,
        "config": {}
      }
    },
    "slots": {
      "memory": "builtin-memory",
      "context_engine": "legacy"
    }
  }
}
```

关键点：

- `allow` / `deny` 做全局控制
- `load_paths` 支持显式加载目录或包
- `entries.<id>.config` 存插件私有配置
- `slots` 给互斥能力使用

## 建议的发现顺序

建议：

1. `plugins.load_paths`
2. `workspace/plugins`
3. `~/.agent-diva/plugins`
4. bundled plugins

这里不建议继续沿用“只放在 channel 目录下”这类做法，因为会让非 channel 插件没有统一归属。

## 实施方式重评估

## 不建议 Phase 1 直接做 Rust 动态库插件

原因：

- ABI 不稳定
- 跨版本兼容成本高
- 崩溃隔离差
- 调试和发布复杂

## 建议 Phase 1 采用“外部进程插件宿主”

推荐方向：

- 插件通过 manifest 描述自己支持哪些 capability
- 插件实际实现为外部进程
- 与 `agent-diva` 通过稳定协议通信
  - 首选 `stdio`
  - 可选本地 HTTP / Unix socket

优点：

- 语言无关
- 崩溃隔离更好
- 权限边界更清晰
- 与现有 MCP 心智更接近
- GUI/Manager 只读元数据时无需执行插件主体

缺点：

- 性能不如进程内
- 协议设计成本更高

综合判断：对 `agent-diva` 更合适。

## 建议 Phase 2 再评估 WASM

如果未来需要：

- 更强隔离
- 更标准化分发
- 更细粒度 capability 授权

可以在稳定 manifest 和 registry 之后评估 WASM component 模式。

但不建议在 Phase 1 就把实现方式押注到 WASM，否则容易拖慢真正的插件架构落地。

## 建议的系统分层

建议在 `agent-diva` 中形成如下职责划分：

- `agent-diva-core`
  - 插件 manifest schema
  - 插件配置 schema
  - capability 枚举
  - slots 定义
  - 安全策略与诊断类型
- `agent-diva-plugin-host`（建议新 crate）
  - 插件发现
  - 插件注册表
  - 宿主通信协议
  - 生命周期管理
- `agent-diva-channels`
  - 消费 `channel` bucket
- `agent-diva-providers`
  - 消费 `provider` bucket
- `agent-diva-tools`
  - 消费 `tool` bucket
- `agent-diva-manager`
  - 暴露插件 inventory / diagnostics / config 状态
- `agent-diva-gui`
  - 展示插件状态、启停、配置 schema 渲染入口

## 对当前路线的建议

## 结论：仍然先做插件，但不要先做 channel-only

推荐调整为：

### P0

- 完成通用插件机制设计文档
- 明确 manifest、bucket、slots、发现顺序、安全边界
- 明确 Phase 1 走外部进程宿主而不是动态库

### P1

- 做最小通用 registry
- 先开放 `channel/provider/tool/service`
- 做 CLI 和 manager 的 `plugins list / inspect / status` 文档和接口设计

### P2

- 再补 `memory/context_engine` slot
- 再补 `media_understanding`、`web_search_provider`、`sandbox_backend`
- 最后再评估 WASM

## 建议的后续文档拆解

如果按本文路线推进，下一批文档建议拆成：

1. `plugin-manifest-spec.md`
2. `plugin-host-protocol.md`
3. `plugin-registry-and-slots.md`
4. `plugin-security-model.md`
5. `plugin-gui-manager-surface.md`

## 本文结论

基于 `.workspace/openclaw` 的实现方式，`agent-diva` 的插件机制不应定义成“channel plugin 功能”。

更合适的定义是：

> 一个统一 manifest 驱动、按 capability bucket 注册、支持 slot、具备安全边界的通用插件平台。

在这个平台里，channel 只是第一批消费方之一，而不是插件系统本身。

## 参考证据

- `.workspace/openclaw/docs/tools/plugin.md`
- `.workspace/openclaw/src/plugins/discovery.ts`
- `.workspace/openclaw/src/plugins/loader.ts`
- `.workspace/openclaw/src/plugins/runtime.ts`
- `.workspace/openclaw/src/plugins/registry.ts`
- `.workspace/openclaw/src/plugins/types.ts`
- `.workspace/openclaw/src/plugin-sdk/core.ts`
- `.workspace/openclaw/extensions/openshell/package.json`
- `.workspace/openclaw/extensions/openshell/index.ts`
- `.workspace/openclaw/extensions/nvidia/package.json`
- `.workspace/openclaw/extensions/nvidia/index.ts`
- `.workspace/openclaw/test/plugin-extension-import-boundary.test.ts`
- `docs/dev/migration.md`
- `docs/dev/2026-03-26-nanobot-gap-analysis.md`
