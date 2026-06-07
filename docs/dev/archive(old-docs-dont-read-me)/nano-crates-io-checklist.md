# Nano Crates.io Publish Checklist

## 1. 目标

本文回答一个执行层问题：

- 如果要让 `agent-diva-nano` 不再通过 monorepo `path` 依赖主仓 crate，而是直接依赖 crates.io 上发布的 `agent-diva-*` 包，当前应该怎么做？

本文是 [nano-runtime-packaging-plan.md](./nano-runtime-packaging-plan.md) 的补充：

- 前者回答“方向是否成立、最终边界应该长什么样”。
- 本文回答“现在能先做什么、先发什么、切换顺序是什么、哪些点不能跳过”。

## 2. 当前状态判断

截至 2026-04-27，`.workspace/agent-diva-nano/Cargo.toml` 仍是如下模式：

- `agent-diva-core = { version = "0.4.10", path = "../../agent-diva-core" }`
- `agent-diva-agent = { version = "0.4.10", path = "../../agent-diva-agent" }`
- `agent-diva-providers = { version = "0.4.10", path = "../../agent-diva-providers" }`
- `agent-diva-tools = { version = "0.4.10", path = "../../agent-diva-tools" }`
- `agent-diva-tooling = { version = "0.4.10", path = "../../agent-diva-tooling" }`
- `agent-diva-files = { version = "0.4.10", path = "../../agent-diva-files", optional = true }`

这说明：

1. `agent-diva-nano` 还没有切到 crates.io 消费模式。
2. 当前最小内部闭包已经明显小于“主产品全闭包”，但仍然偏宽。
3. “先发布现有闭包，再让 nano 按版本引用”在工程上可行。
4. 这条路适合作为过渡态，不应被误认为最终稳定边界。

## 3. 当前最小可发布闭包

按当前代码实际依赖，`agent-diva-nano` 的最小内部闭包如下：

1. `agent-diva-files`
2. `agent-diva-core`
3. `agent-diva-tooling`
4. `agent-diva-providers`
5. `agent-diva-tools`
6. `agent-diva-agent`
7. `agent-diva-nano`

依赖关系可简化为：

```text
agent-diva-files
    ^
    |
agent-diva-core <----- agent-diva-tooling
    ^                      ^
    |                      |
    +---- agent-diva-providers
    |
    +---- agent-diva-tools ----> agent-diva-files
    |              ^
    |              |
    +---- agent-diva-agent -----+
                   ^
                   |
            agent-diva-nano
```

这条闭包里目前不包含：

- `agent-diva-manager`
- `agent-diva-cli`
- `agent-diva-channels`
- `agent-diva-service`
- `agent-diva-gui`

这也是当前最适合先切 crates.io 的原因：`nano` 已经不再依赖 manager/CLI 主线。

## 4. 推荐的两阶段策略

### 4.1 阶段 A：先让当前闭包可发布、可消费

目标：

- 不改变大架构，只把现有 `path` 依赖切成 version 依赖。
- 验证 `agent-diva-nano` 是否真的可以从仓外直接消费主仓已发布 crate。

适用场景：

- 你希望尽快验证“nano 外部化 + crates.io 供给”这条链路是否跑通。
- 你接受这是过渡态，而不是最终产品边界。

### 4.2 阶段 B：收口共享边界后再稳定发布

目标：

- 先抽共享层，例如 `agent-diva-runtime`、`agent-diva-control-plane`。
- 再把 `nano` 对多 crate 的直接依赖收窄成“薄壳 + 稳定运行时面”。

适用场景：

- 你要把 `nano` 当长期独立项目维护。
- 你希望减少未来跨仓版本联动和 API 漂移。

结论：

- 如果目标是“尽快验证能不能跑通”，走阶段 A。
- 如果目标是“形成长期稳定外部生态面”，阶段 B 才是终局。

## 5. 当前建议的发布顺序

按当前 manifest，推荐发布顺序如下：

1. `agent-diva-files`
2. `agent-diva-core`
3. `agent-diva-tooling`
4. `agent-diva-providers`
5. `agent-diva-tools`
6. `agent-diva-agent`
7. `agent-diva-nano`

原因：

- `agent-diva-core` 依赖 `agent-diva-files`。
- `agent-diva-tooling` 依赖 `agent-diva-core`。
- `agent-diva-providers` 依赖 `agent-diva-core`。
- `agent-diva-tools` 依赖 `agent-diva-core`、`agent-diva-files`、`agent-diva-tooling`。
- `agent-diva-agent` 依赖 `agent-diva-core`、`agent-diva-files`、`agent-diva-providers`、`agent-diva-tooling`、`agent-diva-tools`。
- `agent-diva-nano` 依赖上述全部核心闭包。

## 6. 每个 crate 上架前的检查清单

所有发布候选 crate 都应满足以下最低门槛：

1. `Cargo.toml` 没有残留必须依赖 monorepo 相对路径的发布期假设。
2. `cargo package --dry-run -p <crate>` 可以通过。
3. 对外 `README`、`description`、`repository`、`license` 已完整。
4. 没有把纯内部实现细节误暴露成长期公共 API。
5. 新增或关键配置项有文档，不依赖“看源码才知道”。
6. 能解释清楚版本兼容策略，至少同一轮发布内全部内部依赖版本一致。

针对本项目，还应加两条：

1. provider 原生 OpenAI-compatible 端点继续遵守 raw model id 规则，不因 externalization 误改模型名行为。
2. 所有 `agent-diva-*` crate 的版本升级保持同一发布波次同步，不要只发 `nano` 而漏发其闭包依赖。

## 7. 按 crate 的具体判断

### 7.1 `agent-diva-files`

状态：

- 最接近基础层。
- 无内部 crate 依赖。

发布前重点：

- 检查 `sqlx` / 本地文件路径 / 数据目录默认值是否适合作为公开 crate 行为。
- 确认 crate 文档把“这是通用文件管理组件”还是“这是 Agent Diva 内部文件层”讲清楚。

结论：

- 可以作为第一批发布候选。

### 7.2 `agent-diva-core`

状态：

- 是共享域模型和基础能力核心。
- 但它依赖 `agent-diva-files`，因此并不是完全纯 domain crate。

发布前重点：

- 评估 `core -> files` 这条反向依赖是否符合长期语义。
- 若不符合，后续应考虑把更纯的 domain primitive 再向下抽。

结论：

- 当前可以先发。
- 但从长期架构看，仍有继续瘦身空间。

### 7.3 `agent-diva-tooling`

状态：

- 职责较清楚，主要是工具 trait 和 registry primitive。

发布前重点：

- 明确哪些 trait 是承诺给外部实现者的稳定面。
- 避免后续频繁修改 trait 签名导致所有外部工具实现一起破。

结论：

- 适合优先发布。

### 7.4 `agent-diva-providers`

状态：

- 已经具备相对独立的 provider 抽象与实现。

发布前重点：

- 保持 native-provider 与 LiteLLM/gateway 路由语义清晰。
- 确认配置字段、模型发现接口、错误语义足够稳定。

结论：

- 可以进入第一波，但要把 provider 契约当成公开面维护。

### 7.5 `agent-diva-tools`

状态：

- 已有自己的工具实现和 MCP 相关能力。
- 依赖面比前几项更宽。

发布前重点：

- 梳理哪些 built-in tools 是外部用户真正需要稳定依赖的。
- 警惕把“主仓内部工具装配细节”直接固化成公开 API。

结论：

- 可以为了支撑 nano 先发。
- 但后续更理想的状态仍是由更高层 runtime crate 统一装配。

### 7.6 `agent-diva-agent`

状态：

- 是当前 nano 闭包里最宽、最容易演进的 crate 之一。

发布前重点：

- 明确哪些入口函数、运行时控制、skills/toolset 注入面是真正对外承诺的。
- 不能把仍在快速迭代的内部调度细节一股脑暴露出去。

结论：

- 当前为了跑通 nano 仍可能需要先发。
- 但这是最值得后续用 `agent-diva-runtime` 吸收掉的一层。

### 7.7 `agent-diva-nano`

状态：

- 当前已经是较轻的独立壳。
- 但仍直接吃多个内部 crate。

发布前重点：

- README 要明确“这是 starter/template line，不是正式主产品线 CLI”。
- 需要给出仓外消费示例，而不再写 monorepo-only 的构建叙事。

结论：

- 应最后发布。

## 8. `agent-diva-nano` 切换到 crates.io 的落地步骤

### 8.1 第一步：先保证所有依赖 crate 都能独立 `cargo package --dry-run`

建议命令顺序：

```powershell
cargo package --dry-run -p agent-diva-files
cargo package --dry-run -p agent-diva-core
cargo package --dry-run -p agent-diva-tooling
cargo package --dry-run -p agent-diva-providers
cargo package --dry-run -p agent-diva-tools
cargo package --dry-run -p agent-diva-agent
```

如果这里任何一步失败，不要先改 `nano`。

### 8.2 第二步：按顺序真实发布闭包

建议顺序同第 5 节。

发布时要做两件事：

1. 发布后等待 crates.io 索引可见。
2. 再发下一层，不要连续盲推。

现仓库已有 `scripts/wait-crates-io-version.sh`，可以用来等索引出现。

### 8.3 第三步：修改 `agent-diva-nano` manifest

将 `.workspace/agent-diva-nano/Cargo.toml` 从：

```toml
agent-diva-core = { version = "0.4.10", path = "../../agent-diva-core" }
```

改成：

```toml
agent-diva-core = "0.4.10"
```

同理处理：

- `agent-diva-agent`
- `agent-diva-providers`
- `agent-diva-tools`
- `agent-diva-tooling`
- `agent-diva-files`

建议首次切换时保守一点：

- 先在独立分支保留一份 `path` 版 manifest 备份。
- 仅在 `nano` 仓或 staging 目录切换，不回写主产品线依赖图。

### 8.4 第四步：从仓外环境验证

最低要求不是“在 monorepo 里还能编”，而是：

1. 在不依赖主仓相对路径的环境中 `cargo check` 通过。
2. `cargo test` 至少通过 `nano` 自身关键测试。
3. 示例代码可以按 README 独立运行。

如果只在 monorepo 内能过，不能算真正切换完成。

## 9. 推荐的验证清单

### 9.1 阶段 A 最低验证

1. 根 workspace：
   - `just fmt-check`
   - `just check`
   - `just test`
2. 发布闭包：
   - 每个候选 crate 执行 `cargo package --dry-run -p <crate>`
3. nano staging：
   - `cargo check --manifest-path .workspace/agent-diva-nano/Cargo.toml`
   - `cargo test --manifest-path .workspace/agent-diva-nano/Cargo.toml`
4. 仓外 smoke：
   - 在独立目录创建最小 demo，直接依赖 crates.io 上的 `agent-diva-nano`

### 9.2 阶段 B 附加验证

若后续引入 `agent-diva-runtime` / `agent-diva-control-plane`，则要补：

1. 共享 runtime crate 的单测与集成测试。
2. manager 和 nano 对共享层的双端 smoke。
3. HTTP 控制面契约回归，尤其是事件流和配置热更新路径。

## 10. 当前不建议直接做的事

以下动作当前不建议直接做：

1. 还没抽稳共享边界，就把大量内部 crate 一次性宣传成公共稳定 API。
2. 只发布 `agent-diva-nano`，但不发布或不同步发布它的闭包依赖。
3. 只在 monorepo 内做 `path -> version` 替换测试，就宣布 externalization 完成。
4. 把 `nano` 再重新塞回主 workspace，试图用本地便利掩盖真实发布问题。
5. 把 `agent-diva-manager` / `agent-diva-cli` 再引回 nano 闭包。

## 11. 最终建议

如果目标是“现在就先验证能不能让 nano 吃 crates.io 包”，推荐执行顺序是：

1. 先按当前最小闭包发布：
   - `files -> core -> tooling -> providers -> tools -> agent`
2. 再把 `agent-diva-nano` 改成纯 version 依赖。
3. 从仓外环境做真实构建与 smoke。
4. 跑通后，把这条链路视为过渡态成果。
5. 下一阶段再推进 `agent-diva-runtime` / `agent-diva-control-plane` 收口。

如果目标是“做长期稳定的 nano 外部项目”，则推荐顺序是：

1. 先把共享 runtime/control-plane 边界抽出来。
2. 再重排发布闭包。
3. 最后再把 `nano` 固化为对稳定共享层的薄壳。

一句话总结：

- 现在已经可以开始准备“让 nano 直接引用 crates.io 包”。
- 但正确做法是先把当前最小闭包作为过渡发布链路跑通，再继续收口边界，而不是把当前宽闭包直接当成最终架构。
