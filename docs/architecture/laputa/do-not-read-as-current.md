# 不得再当现行 Laputa 依据

下列说法一旦当作「现在该怎么做」，就会和 2026-08 决策冲突。原文可留作考古，不能指导实施。

| 旧说法 | 出现位置（典型） | 现行 |
| --- | --- | --- |
| Laputa = 14 content section + `.laputa/state.json` | 根目录旧 `LAPUTA.md`（2026-06-12 v0.0.6） | 七份 Markdown 权威 + 独立 BML + 独立 ACTMEM |
| Identity / Commitment / Preferences / memory_md | 旧 PRD、旧 JSON section、旧 GUI 左栏 | `IDENTITY` / `REDLINE` / `USER`；删除 `memory_md` |
| `MEMORY.MD` = 长期记忆或 STM 文件 | 旧 Garden、旧 Laputa section | 禁止核心文件名；STM 概念 ≠ 文件；注入文件若落地叫 `ACTMEM.MD` |
| `STM.MD` 作为核心文件 | 远古 UPSP | 禁止；STM/LTM 只是概念称呼 |
| Mentle 读、Laputa 写 | 旧 LAPUTA.md §0 | Mentle 已退役；BML 是 LTM 唯一权威 |
| AutoDream 是 Evolution 收件箱 / MemoryPatch 主链 | 旧 Evolution、当前部分代码 | 产品：整理 ACTMEM；人格按 P19 提案；禁止 MemoryPatch 当主职 |
| WORLD 有界投影动态装进 Prompt | R3 / 旧 P20 初稿 | WORLD 走工具，不动态加载 |
| 每个 git 项目一套人格 | 旧 workspace `.laputa` 直觉 | 一个 Diva 一套人格，七份同一目录 |
| SessionCheckpoint / `working_memory` = STM | Wave 2 实现 | 不是；寿命不同 |
| 人格变更走 Governance / Approval Center | 旧 GUI | P5 内容审查；P16 直写例外 |
| 根目录旧 `USER.md`（retire 源）= 新 `USER.MD` | 文件名撞车 | 不是同一文件 |
| MEMRULES 是 Laputa 认知文件 / Persona 左栏只读 | `.laputa/cognitive/MEMRULES.MD`、GUI `memrules`+`world` 组 | **S9/P21**：手册在 `{config_dir}/memory/`；Memory 设置可编；不进人格 |
| MEMRULES 整本常驻 Prompt，或永远不给写记忆的模型看 | 旧 `context_plane_invariants` 一刀切 | 日常禁全文；写记忆时注入（对标 GA L0） |
| 一台机器多个 Diva profile / 多套人格约会 | 旧「一份 profile 一套」字面 | **P22**：整机一份伴侣 |
| BML 按 git 仓库各开一套 | 工作区 `.laputa/memory.sqlite3` | **S1 修订**：跟人格走 |
| `memory_distill` 新建 Skill 静默直写 | `typed_provider` 现行 | **D7**：一律 Evolution 人审 |
| 空闲 10 分钟才第一次归纳 STM | 旧 S8 | **S8 修订**：每轮立刻 Recap；10 分钟只折叠 |

决策摘要若与 6 月 `docs/decisions/laputa-memory-governance-history-2026-06.md` 冲突，以 8 月决策和本目录 `architecture.md` 为准。
