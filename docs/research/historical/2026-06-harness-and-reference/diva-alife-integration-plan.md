# DIVA × ALIFE 整合调研报告 —— "这波怎么做"

> 主进程综合产出 · 2026-06-18
> 输入材料:
> - SA1 [`alife-plugin-module-di.md`](./alife-plugin-module-di.md) —— Plugin/Module + DI 地基
> - SA2 [`alife-function-skill-memory.md`](./alife-function-skill-memory.md) —— FunctionCaller + Skill + Memory 能力层
> - SA3 [`alife-proactive-selfupgrade.md`](./alife-proactive-selfupgrade.md) —— SystemEvent + Developer + VirtualWorld 前沿层
> - `agent-diva-pro/DECISION.md`(11 条 alife 特性处置)
> - `agent-diva-pro/docs/architecture/evo-diva-architecture-2026-06-12.md`(Authority Spine)
> - `morediva/papers/` 下 7 篇论文
> - diva-pro 已读源码: planning/ 5200 LOC, heartbeat 766 LOC, autodream/rhythm, laputa

---

## 0. TL;DR(给"忙碌的设计负责人")

**alife 的核心价值不是它的某一处实现,而是它的三件事能拆开**:
- **Plugin/Module + DI** —— alife 是单层抽象,但 Rust 这边能借鉴其 DI 装配思想;Roslyn 热重载不可移植
- **Skill 按需加载** —— alife 显隐分桶的中间档 diva 缺,**这块最该先做**
- **Proactive UpdatePrompt** —— alife "不要告诉主人是定时器"的核心 UX,diva heartbeat 改造方向

**这波建议聚焦 3 件事**(按优先级):
1. **Skill 按需加载 PoC**(1 周) —— 改 1 工具 + 1 renderer 分支,零破坏
2. **Heartbeat 2.0(alife 式主动报点)**(3-4 周) —— 替换 766 LOC 的固定 cron,加入 UpdatePrompt + 指数退避
3. **Module trait 设计 + WASM 插件 spike**(6-8 周) —— 为 DECISION #10/#11 铺路

**3 个不该现在做**(后续 wave 再说):
- 把 XmlFunctionCaller 整套移植 —— diva 走 OpenAI 兼容 tool calling 是对的,只取"显隐分桶"思想
- 记忆层级压缩 —— Laputa 已有提案/合并语义,alife 的 `Filter` 会绕过 Laputa,先设计再决定
- 多开/赛博世界 —— DECISION #9 明确 defer,VirtualWorld 借鉴价值低

---

## 1. 我们现在的位置

### diva-pro 已有家底(代码层)

| 模块 | 规模 | 对应 Park 论文 | 状态 |
|------|------|----------------|------|
| `agent-diva-laputa` | 权威存储 + changelog + audit + rollback | Memory Stream + 治理 | ✅ |
| `agent-diva-autodream` rhythm | 跨会话反思 + 日/周报 | Reflection 柱 | ✅ |
| `agent-diva-agent/planning/` | 5200 LOC,5 层 planning + NAG | Planning 柱 + Initiative 触发 | ✅ |
| `agent-diva-core/heartbeat/` | 766 LOC,2 阶段 Decide→Execute | 主动唤醒 | ✅(待 alife 化) |
| `agent-diva-core/cron/` | 1275 LOC,时序任务 | - | ✅ |
| Authority Spine(EVO-DIVA) | EvidenceRef→Proposal→user review→Laputa apply | - | ✅(diva 独有) |

**核心结论**: diva-pro 已有 Park 2023 Generative Agents 四柱 + DECISION #6/#10 所需的全部基础设施。**真正"临门一脚"的是把已有能力组合成"自主活动"产品形态**,以及新增"主动活动设计决策"。

### alife 调研发现的 6 个最有价值点

| 借鉴点 | 难度 | 价值 | 建议 |
|--------|------|------|------|
| Skill "列出但按需读" 显隐分桶 | 🟢 低(1 周) | 🔴 高(解锁"工具/技能不爆 system prompt") | **P0 这波** |
| Heartbeat "UpdatePrompt + 不要告诉主人 + 指数退避" | 🟡 中(3-4 周) | 🔴 高(自主活动产品形态) | **P0 这波** |
| Module DI 构造签名 + 三段生命周期 | 🟡 中(2-3 周) | 🟡 中(扩展性,但 agent-diva-tools 已够用) | 推后 |
| Module 自带 UI(EditorUI trait) | 🟡 中 | 🟡 中(可选) | 推后 |
| Tool on-demand trigger(DocumentMode) | 🟡 中 | 🟢 中低(目前工具数 < 30 不急) | 推后 |
| VirtualWorld 共享 SP | 🟡 中 | 🟢 低(DECISION #9 defer 多开) | 不做 |

---

## 2. 这波的 3 个目标

### 目标 1:Skill 按需加载 PoC

**目的**: 解决"工具/技能数量增长时 system prompt 爆炸"的最简方案。

**做法**(SA2 §B 提炼):
- diva-pro 已有 `agent-diva-agent/src/skills.rs` 里的 `SkillsLoader`(解析 SKILL.md frontmatter,`always: bool` 字段)
- 现状:`always=false` 的 skill **完全不注入** system prompt,AI 看不到
- 改造:渲染 system prompt 时,`always=false` 改注入 `(name, description)` 摘要行;加 1 个内置 `study_skill(name)` 工具,server 端读 `workspace/skills/<name>/SKILL.md` 并 Poke 回对话上下文

**关键文件**:
- 新增:`agent-diva-tools/src/skill_tools.rs`(study_skill 实现)
- 改:`agent-diva-agent/src/skills.rs`(renderer 分支)
- 不动:laputa / autodream

**完成标准**:
- [ ] 在 system prompt 渲染时,on_demand skill 只显示 `(name, description)`
- [ ] AI 能调 `study_skill("github")` → server 把 SKILL.md 推回
- [ ] 1 个 PoC skill(随便挑 1 个)走通流程

**工作量**: 1 周(2-3 天 + 1 周验收)

### 目标 2:Heartbeat 2.0 —— alife 式主动报点

**目的**: 把 diva heartbeat 从"被动 cron 轮询"改成"主动活动许可",这是 DECISION §6 的第一步。

**核心改造**(SA3 §A 提炼):
- alife `SystemEventService` 三段 Prompt 模式:
  - `StartPrompt` = 重启时注入"(状态已重置)"
  - `DestroyPrompt` = 关闭时注入"(系统关闭,只可道别)"
  - `UpdatePrompt` = **核心**:"**不要告诉主人有自动报点**…主动找主人玩、看新闻…"
- alife `IsIdle + 指数退避 + 自由提示词` 三件套:
  - 默认 90s + 随机±30s
  - 连续静默 → 间隔 ×3^n,封顶 retry=4(90s → 270s → 810s → 2430s)
  - 用户/AI 发言 → 重置计数
- alife `EWake / EWait`:AI 可调 `schedule_wake(iso, remark)` / `wait(seconds)` 自我调度

**diva-pro 改动**:
- `agent-diva-core/src/heartbeat/types.rs`:
  - `HEARTBEAT_SYSTEM_PROMPT` 改写为中文版 UpdatePrompt
  - 加 `default_interval=90s`、`backoff_factor=3.0`、`max_retry=4` 配置
  - 加 `continuous_silent_count` 状态
- `agent-diva-core/src/heartbeat/service.rs`:
  - `decide()` 阶段读"是否 IsIdle" + 算下次间隔
  - `OnChatSent` hook 重置 silent_count
- `agent-diva-tools/src/wake_tools.rs`(新):
  - `schedule_wake(iso, remark)` / `wait(seconds)` 实现
- `agent-diva-gui`:Settings 加 heartbeat 间隔/退避配置

**关键设计问题**(本波必须回答):
- **Q1:heartbeat 决定"主动找主人"时,走哪个 channel?**
  - 候选:写 Laputa(纯后台)/ push 静默 notification(系统通知)/ 主动发起对话(打破沉默)
  - **建议**:默认写 Laputa + 显示 GUI inbox 角标,**不主动发起对话**(避免打扰)。可配置"高优先级时 push 通知"。
- **Q2:UpdatePrompt 触发 LLM 的 token 成本**
  - 每 90s 一次,保守估计每天 ~960 次 → 需估算 LLM 单价,可能要做"轻量决策模型"代理
- **Q3:走 Authority Spine 吗?**
  - 答案:**不走**(心跳是行为,不是 authority 写入)
  - 但心跳**配置**(间隔、UpdatePrompt 文案)走 Spine(用户能 review "我现在心跳在做 X")

**完成标准**:
- [ ] heartbeat service 跑 UpdatePrompt + 指数退避
- [ ] AI 能用 `schedule_wake` / `wait` 工具
- [ ] Settings 页面有 heartbeat 配置(走 Spine 写入)
- [ ] 1 周 soak test:heartbeat 触发频率、token 消耗、用户打扰度

**工作量**: 3-4 周(2 周改造 + 1-2 周 soak)

### 目标 3:Module trait 设计 + WASM 插件 spike

**目的**: 为 DECISION #10(自我升级)+ #11(插件系统)铺路。这一波**只做设计 + POC**,不做完整实现。

**设计层面**(SA1 提炼):
- 新建 `agent-diva-module` crate,定义:
  ```rust
  #[module(name, category, launch_order)]
  pub trait Module: Send + Sync {
      async fn awake(&mut self, ctx: &ModuleContext) -> Result<()>;
      async fn start(&mut self, ctx: &ModuleContext) -> Result<()>;
      async fn destroy(&mut self) -> Result<()>;
  }
  ```
- `Module` 自动注册过程宏(扫所有 `impl Module for X`)
- 给 `agent-diva-tools::Tool` trait 加 `launch_order: i32` + `category: &str` 字段(零侵入改动,默认 0 / "general")

**POC 层面**(SA3 §B 提炼):
- alife 的 `pluginRoot / pluginCopyRoot` 双目录隔离思想 → Rust 这边:
  - `~/.diva-pro/plugins/` 真实(AI 可编辑)
  - `target/wasm/` 编译产物(每次 reload 重建)
- AI 编辑 plugins 下 Rust 源 + 触发 `cargo build --target wasm32-wasip2` + `wasmtime::Store` drop/replace
- **POC 范围**:1 个最简单的 Module 走通"AI 改源码 → reload → 上线"完整流程
- **不做的**:完整 IDE-like 工具集、撤销/回滚(Authority Spine 已经兜底)

**关键文件**:
- 新增:`agent-diva-module/` crate(空骨架 + trait 定义)
- 改:`agent-diva-tools/src/lib.rs`(加 2 个字段)
- 新增:`agent-diva-pro/agent-diva-tools/src/plugin_dev.rs`(reload_module 工具)
- 新增:`docs/architecture/module-trait-design.md`(设计文档)

**完成标准**:
- [ ] Module trait 定义 + 过程宏可用
- [ ] 1 个模块走通 WASM 编译/实例化/卸载/重载完整路径
- [ ] 设计文档说明 Module 与 Authority Spine 的边界(什么走 Spine 什么不走)

**工作量**: 6-8 周(2 周设计 + 2-3 周 POC + 2 周验收),**本波只承诺"设计 + POC 跑通"**,完整功能推下一波。

---

## 3. 风险与不做清单

### 不该这波做的 4 件事

| 不做项 | 理由 | 何时可重提 |
|--------|------|------------|
| XmlFunctionCaller 移植 | diva 走 OpenAI 兼容,XML 流式是 SK 时代产物,迁等于重写 providers | 永不(diva 已选 OpenAI tool calling) |
| 多级 cache 记忆压缩 | alife `Filter` 会绕过 Laputa,需先 design doc | 等"工作台 PRD 化"完成后 |
| Roslyn 式运行时编译 | Rust 无 JIT 等价物 | wasmtime 已是最优解 |
| VirtualWorld 多开互联 | DECISION #9 明确 defer | 永不(单一人格哲学) |

### 3 个潜在风险

1. **Authority Spine 的扩展性**: 这波 3 个目标都可能新增"走 Spine"的对象(heartbeat 配置、Module 注册、Skill 注册)。需先确认 Spine 当前实现能扩展
2. **token 成本**: Heartbeat 2.0 每 90s 触发 LLM,token 消耗可观,需要先做成本估算
3. **GUI Settings 字段**: heartbeat / Module / Skill 三套配置都要进 GUI,涉及 Vue 3 + Tauri v2 改动,可能拖累节奏

---

## 4. 建议时间线(2 个月)

```
W1        W2        W3        W4        W5        W6        W7        W8
├─────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┤
│ ████ Skill PoC ████│  ↑ 验收 ↑       │                                  │
│                  │ ████ Heartbeat 2.0 ████████████████│ ↑ soak ↑         │
│                  │                  │ ████ Module trait + WASM POC ████│ ↑ 验 ↑
│                  │                  │                  │  ↑ design review ↑
└─────────────────┴──────────────────┴──────────────────────────────────┘
```

- **W1-W2**: Skill 按需加载 PoC(目标 1)
- **W3-W6**: Heartbeat 2.0 改造 + soak(目标 2)
- **W3-W8**: Module trait + WASM POC,跟 heartbeat 并行(目标 3)
- **W8 末**: 3 个目标一起验收

### 验收点

- **W2 末**: Skill 按需加载 PoC 演示
- **W6 末**: Heartbeat 2.0 跑通 + 1 周 soak 数据
- **W8 末**: Module trait WASM POC 跑通 + 设计文档交付

---

## 5. 给用户(设计负责人)的下一步建议

### 立即可做的 3 件准备

1. **过一遍这份报告** —— 哪些点你不同意?哪些要砍?哪些要前置?
2. **决定 Q1(heartbeat 主动行为的 channel 出口)** —— 这是产品 UX 决策,不是技术决策
3. **预算 W2-W8 的 token 成本** —— Heartbeat 2.0 的 LLM 调用是固定开销,要确认值不值

### 我建议的工作顺序

1. 先把 **目标 1(Skill PoC)** 做了 —— 1 周内可见,验证 alife 借鉴路径可行
2. 同时启动 **目标 3 的设计部分**(Module trait 草案),不写代码,只写文档
3. Skill PoC 验收后,**再决定**目标 2 / 目标 3 哪个先做完整实现

如果这 3 件都同意,我就开始起 todo + 排具体任务。如果有不同意,我改完再提交。
