# agent-diva 记忆框架实施方案

## 1. 文档目标

这份文档回答四个问题：

1. 具体要做什么
2. 技术架构怎么走
3. 如何确保最小化演进
4. 如何确保解耦

本文是实施方案，不是愿景文档，也不是代码实现。它的目标是让后续开发可以按阶段落地，而不会失控扩张。

## 2. 总体目标

本次实施主线不是“加一个 diary 功能”，而是：

> 为 `agent-diva` 建一套可持续扩展、能承接 `zeroclaw` / `openclaw` 核心理念的记忆框架。

在这个框架中，本阶段唯一准备真正落地的实验功能是：

- `rational diary storage`

也就是：

- 记录项目分析
- 记录文档入口
- 记录阶段性判断
- 记录下一步建议

但 diary 只是实验入口，不是系统目标本身。

## 3. 本次实施包含什么

## 3.1 必须完成的设计工作

### A. 记忆框架边界设计

明确区分：

- session history
- durable memory
- diary system
- future recall engine
- prompt recall policy
- soul / identity / relationship integration

### B. 抽象接口设计

明确未来要有的抽象层：

- `MemoryStore`
- `DiaryStore`
- `RecallEngine`
- `MemoryToolContract`

### C. 存储布局设计

明确：

- 文件布局
- diary 路径
- metadata 字段
- 未来 database mapping

### D. 演进路径设计

明确每个阶段做什么、不做什么，以及完成条件。

## 3.2 本阶段唯一实验性功能

只准备后续实现：

- 理性日记基础存储

范围限定为：

- 创建目录
- 按天归档
- 追加半结构化条目
- 为未来 recall 保留字段

不包含：

- emotional diary
- diary search
- embeddings
- hybrid retrieval
- soul 自动演化

## 4. 技术架构怎么走

## 4.1 采用“分层承接”而不是“一次性替换”

推荐总路径：

```text
现有 session + MEMORY.md
  -> 增加结构化抽象
  -> 增加 diary domain
  -> 增加 future recall slot
  -> 再逐步替换全量 prompt 注入
```

这意味着前几步不是重写现有系统，而是在现有系统旁边建立可替换的新层。

## 4.2 目标分层

### Layer 1: Session Layer

保留当前 `SessionManager`。

职责：

- 保存即时会话
- 提供 consolidation 输入

这一层短期不改协议、不改存储格式。

### Layer 2: Durable Memory Layer

未来目标：

- 不再只靠 `MEMORY.md`
- 引入统一 memory abstraction

但实施顺序上先做抽象设计，不立即替换现有 `MemoryManager`。

### Layer 3: Diary Layer

本阶段唯一真正落地方向。

职责：

- 存储理性分析型日记
- 记录过程性沉淀
- 成为 future recall source 之一

### Layer 4: Recall Layer

本阶段只定义接口，不实现。

未来职责：

- 按 query 主动召回
- 控制 prompt 预算
- 替代当前全量 `MEMORY.md` 注入

### Layer 5: Soul Integration Layer

本阶段只设计边界。

未来职责：

- 接收稳定记忆信号
- 驱动 `SOUL.md` / `IDENTITY.md` / `USER.md` / `RELATIONSHIP.md`

## 4.3 crate 路径怎么走

## 第一阶段建议

### `agent-diva-core`

只增加“稳定基础类型”和路径约定：

- memory domain enums
- diary partition enums
- diary entry struct
- path helpers

原因：

- core 适合放稳定领域模型
- 不适合放复杂检索引擎

### `agent-diva-agent`

后续只承接：

- diary 提炼逻辑
- 何时写入
- 与 session/consolidation 的协同

### `agent-diva-tools`

后续为外部调用暴露：

- `diary_write`
- `diary_read`
- `diary_list`

### 新 crate 预留：`agent-diva-memory`

真正的 memory engine 单独放。

不要把未来 recall/索引/embedding 继续堆进 `agent-diva-core` 或 `agent-diva-agent`。

## 5. 实施阶段拆解

## Phase 0：文档与边界冻结

目标：

- 固定术语
- 固定模块边界
- 固定不做项

产出：

- 综合设计
- Phase A 规格
- 能力对齐规划
- 本实施方案

完成标志：

- 后续开发不会再把“diary”误当成系统目标

## Phase 1：基础抽象落地

要做：

- 增加 memory / diary 领域类型
- 增加 diary 路径约定
- 增加 diary 条目格式规范

不做：

- recall engine
- diary search
- emotional partition

完成标志：

- diary 成为正式 memory domain，而不是临时 markdown 旁路

## Phase 2：理性日记存储落地

要做：

- `memory/diaries/rational/YYYY-MM-DD.md`
- 追加条目能力
- 半结构化 markdown 模板

不做：

- 向量检索
- 自动魂系演化
- 感性分区

完成标志：

- agent 能稳定把分析型结论写入 rational diary

## Phase 3：结构化 MemoryStore 抽象

要做：

- 统一 memory store 接口
- diary store 接口
- 与现有 `MemoryManager` 的适配关系

不做：

- 直接替换所有旧路径

完成标志：

- diary / stable memory 不再是两套完全独立思路

## Phase 4：Recall 接口接入

要做：

- recall engine trait
- prompt recall slot
- tool contract slot

不做：

- embeddings/hybrid 的完整实现

完成标志：

- 全量 MEMORY 注入模式开始可以被替代

## Phase 5：能力增强

后续才考虑：

- emotional diary
- embeddings
- hybrid retrieval
- soul governance automation
- relationship memory

## 6. 如何确保最小化演进

## 6.1 保持旧路径可用

最小化演进的第一原则：

> 先加层，不换核。

意思是：

- 不先删 `MemoryManager`
- 不先改 `ContextBuilder` 的全部行为
- 不先改 session 存储
- 不先做全系统迁移

而是先把新层加出来。

## 6.2 新能力默认旁挂，而不是侵入替换

例如 rational diary：

- 先作为 `memory/diaries/rational/` 旁挂
- 不先改 `MEMORY.md` 的现有职责
- 不先要求所有 agent 行为都写 diary

## 6.3 每一阶段只引入一个新不变量

建议每阶段只增加一个主要变化：

- Phase 1：领域模型固定
- Phase 2：理性日记可存
- Phase 3：memory store 抽象出现
- Phase 4：recall 接口出现

不要一阶段同时引入：

- 新抽象
- 新后端
- 新工具
- 新 prompt policy

这样风险会叠加。

## 6.4 新层先提供适配器，不要求全量重构

例如未来出现 `DiaryStore` 时，应允许：

- 旧 `MemoryManager` 继续工作
- 新 diary path 通过适配层接入

而不是要求一次性把全部 memory 行为改成统一引擎。

## 6.5 优先保留文件可读性

最小化演进还意味着：

- 初期优先 markdown
- 不急着把一切都推入 sqlite
- 先让人工能审查和纠偏

这对 diary 尤其重要。

## 7. 如何确保解耦

## 7.1 解耦原则 1：domain 与 storage 解耦

不要把“理性日记”直接等同于某种文件格式。

正确关系应是：

```text
DiaryDomain
  -> DiaryStore abstraction
  -> MarkdownDiaryStore implementation
```

这样未来才能换成：

- sqlite-backed diary store
- hybrid diary store

## 7.2 解耦原则 2：memory framework 与 prompt builder 解耦

不要把 recall 逻辑直接写死在 `ContextBuilder` 里。

正确关系应是：

```text
ContextBuilder
  -> RecallOrchestrator
  -> Memory/Diary sources
```

这样未来可以：

- 替换 recall 策略
- 控制不同 source 的注入预算

## 7.3 解耦原则 3：diary 与 soul evolution 解耦

不要让 diary 写入直接触发 soul 变更。

正确关系应是：

```text
Diary
  -> Stable Signals
  -> Governance
  -> Soul Update
```

这能避免：

- 短期波动直接改人格
- 实验功能反向污染主系统

## 7.4 解耦原则 4：工具契约与内部实现解耦

未来工具只暴露契约：

- `diary_write`
- `diary_read`
- `memory_search`

不要把内部文件路径和内部实现细节直接暴露成系统耦合点。

## 7.5 解耦原则 5：实验域与核心框架解耦

本阶段最重要的一条：

> 即便理性日记是首个实验功能，也不能让整体框架围着理性日记建模。

正确建模方式应该是：

- 先定义通用 memory framework
- 再把 rational diary 当成其中一个 domain

## 8. 风险与对应策略

## 风险 1：开发目标被 diary 单点绑架

表现：

- 后续所有抽象都只服务 diary

应对：

- 每份文档都明确 diary 是 experimental domain

## 风险 2：过早引入 recall/embedding 复杂度

表现：

- 一开始就想做 sqlite + embedding + hybrid

应对：

- recall 先只设计接口
- 先稳定存储层

## 风险 3：过早侵入现有 prompt 组装链

表现：

- 还没抽象好就重写 `ContextBuilder`

应对：

- 先引入 recall slot，不替换旧行为

## 风险 4：soul 被实验功能污染

表现：

- 日记中的阶段性判断直接改 soul

应对：

- diary -> soul 之间必须有治理层

## 9. 推荐的实施输出顺序

如果继续只写文档，不写代码，建议后续输出顺序如下：

1. `Memory Framework Interfaces Spec`
2. `Rational Diary File Format Spec`
3. `Memory/Diary Tool Contract Spec`
4. `Prompt Recall Integration Spec`
5. `Migration and Compatibility Spec`

这样写的好处是：

- 每份文档只解决一个问题
- 能直接对应未来开发任务
- 不会把设计压成一篇泛文

## 10. 最终结论

如果用一句话概括这份实施方案：

> 先以最小侵入方式搭出可承接 zeroclaw/openclaw 能力的记忆框架骨架，再只把 rational diary 作为第一个实验性 memory domain 落地。

如果用一句话概括最小化演进策略：

> 先加层、后替换；先抽象、后增强；先实验、后收敛。

如果用一句话概括解耦策略：

> 让 domain、store、recall、prompt、soul governance 各自成层，通过契约连接，而不是直接互相写死。
