---
title: "Alife Feature Disposition — Decision v1"
date: 2026-06-18
status: approved
owner: 大湿
applies_to: agent-diva-pro
source:
  - alife v2.0.0 release: https://github.com/BDFFZI/Alife/releases/tag/v2.0.0
  - morediva/.workspace/alife/WHY.md
supersedes: null
related:
  - docs/architecture/scope-merge-decision.md
  - docs/architecture/evo-diva-architecture-2026-06-12.md
  - docs/prds/prd-laputa-2026-06-12/prd.md
  - docs/prds/prd-autodream-2026-06-12/prd.md
  - docs/prds/prd-selfinprove-2026-06-12/prd.md
  - diva-olv-package/Open-LLM-VTuber (olv2)
  - agent-diva-laputa (已完成)
  - memtle = 0.1.2 (Mentle runtime governance)
  - agent-diva-swarm (defer)
  - selfinprove PRD "Journal 批处理工作台"
---

# DECISION — Alife Feature Disposition (v1)

> **TL;DR**: 对照 alife v2.0.0 的能力矩阵,逐条决定 agent-diva-pro 的处置:不做 / 已完成并超越 / 放到工作台 / 推迟。**核心结论: 多开互联与 Live2D 桌宠不做,长期记忆与脚本执行已是优势,自主活动与自我升级是下一生命周期的主战场,视觉/浏览/插件统一收敛到工作台。**

---

## 1. 决策摘要表(11 条)

| # | alife 能力 | 处置 | 归属/下一站 | 状态 |
|---|------------|------|-------------|------|
| 1 | Live2D 桌宠 | **不做** | 推迟到 **olv2** 完成 | 锁定 |
| 2 | 深度视觉(图像/OCR/屏幕识别) | **工作台** | selfinprove PRD 的 "Journal 批处理工作台" | 待并轨 |
| 3 | 语音对话(TTS/ASR) | **已做** | `agent-diva-providers` transcription | ✅ |
| 4 | 长期记忆 | **已完成 + 超越** | laputa PRD + 自实现 next-gen | 待 v2 |
| 5 | 平台通讯(QQ/Discord 等) | **channel 优化 + defer** | `agent-diva-channels` 优化通道可靠性 | 后续 |
| 6 | 自主活动(自动报点/非线性间隔) | **下一生命周期重大功能** | 架构设计(基于 laputa + Mentle) | 设计待启 |
| 7 | 网上冲浪(agent browser) | **工作台** | 工作台系统(统一入口) | 待并轨 |
| 8 | 脚本执行(Python/Shell) | **已做** | `agent-diva-tools` spawn + sandbox | ✅ |
| 9 | 多开互联(角色互聊/赛博世界) | **不做** | 蜂群已 defer;与单一人格哲学冲突 | 锁定 |
| 10 | 自我升级(AI 编辑插件/热重载) | **下一生命周期重要功能** | 调研(参考 hermes/alife developer module) | 调研待启 |
| 11 | 插件系统 | **重要 + 调研** | 与**工作台**一并设计(不单独立项) | 调研待启 |

---

## 2. 各条理由

### 2.1 Live2D 桌宠 — 不做

- **版权风险**: Live2D 商业授权 + 模型版权两层风险。
- **alv2 路径**: `diva-olv-package/Open-LLM-VTuber` 已存在,等 olv2 完工再评估承接。
- **不替代方案**: GUI 走 Tauri(`agent-diva-gui` 已成型),无桌宠形态需求。

### 2.2 深度视觉 — 工作台

- agent-diva 已有基础视觉能力(`agent-diva-providers` 含视觉模型)。
- 图像识别 / OCR / 屏幕识别属于"重 UI + 重工作流"的能力,不放在主对话流。
- **归属**: selfinprove PRD 已经把 Journal 升级为"批处理工作台",视觉工具是该工作台内的子能力。

### 2.3 语音对话 — 已做

- `agent-diva-providers` 已含 transcription 抽象。
- 无新工作。

### 2.4 长期记忆 — 已完成且超越

- laputa PRD 已落地(记忆候选 → 写入 → 检索闭环)。
- **下一大版本**: 超越 alife 的"多级 cache + bge-small-zh 向量 + DuckDB"路径,做自有下一代长期记忆。
- 不抄 alife 的实现路径,各自演进。

### 2.5 平台通讯(QQ 等)— 优化 channel 后 defer

- 当前 `agent-diva-channels` 多通道已通,主要问题是**通道可靠性**(类比 alife 的 QChatServer onebot 自动重连 + 异常回灌)。
- 决策:**先把现有 channel 做到生产级稳定,新增通道接入 defer**。
- 触点: Alife 的 `QChat` onebot 自动重连 + 异常反馈给 AI 是具体可借鉴的可靠性模式。

### 2.6 自主活动 — 下一生命周期重大功能,待架构设计

- 当前基础设施已完成:
  - `agent-diva-laputa` ✅(记忆/迁移/恢复)
  - `memtle = 0.1.2`(Mentle runtime governance)
- 缺: 在此之上的"agent 自主活动"架构(类比 alife 的 SystemEvent 阶梯定时 + 自动报点提示词引导 AI 主动重置)。
- **下一步**: 起一个自主活动架构设计,明确与 laputa/Mentle 的边界与契约。

### 2.7 网上冲浪(agent browser)— 工作台

- 与视觉(2.2)同源: 重 UI + 重工作流。
- 工作台 = "AI 帮我做事"的统一入口,包含浏览器子能力。

### 2.8 脚本执行 — 已做

- `agent-diva-tools` 的 spawn + `agent-diva-sandbox` 提供受控执行。
- 无新工作。

### 2.9 多开互联(角色互聊)— 不做

- **架构冲突**: agent-diva 哲学是**单一人格**,用户与一个 diva 建立长期关系。
- 多开 = 多角色实例互通,会破坏"唯一人格"的连续性。
- `agent-diva-swarm` 已 defer,与本决策一致。
- 替代: 真要多角色,在工作台里"切换"而非"并存"。

### 2.10 自我升级(AI 编辑插件/热重载)— 重要,调研中,参考 hermes

- 这是 OpenFang 强调的 autonomous agent 核心能力,也是 alife 已落地的能力。
- agent-diva-pro 当前**无此能力**,需要调研。
- **参考对象**:
  - **hermes** —— 配置驱动 / skill / plugin 体系
  - **alife Developer module** —— 热编译热重载
- 短期可走 skill 文件热重载(无需编译),长期是否上 Rust 动态编译待定。

### 2.11 插件系统 — 重要,与工作台一并设计

- 插件系统不是孤立组件,是工作台生态的承载层(用户在工作台里安装/管理/卸载插件)。
- **决策: 插件系统不单独立项,和工作台一起设计**。
- 调研范围:
  - alife Plugin vs Module 二元抽象 + DI 装配
  - hermes 的 plugin / skill 边界
  - nanobot 的 entry-points 自动发现

---

## 3. 横向影响

| 影响面 | 触发 | 动作 |
|--------|------|------|
| `agent-diva-channels` | 2.5 | 起一个 channel 可靠性 backlog(自动重连 + 异常回灌) |
| `agent-diva-autodream` + `agent-diva-laputa` | 2.6 | 起自主活动架构设计,明确 laputa/Mentle 边界 |
| selfinprove PRD | 2.2 / 2.7 / 2.11 | 工作台 PRD 化(目前"Journal 批处理工作台"是隐含概念,需显式升格) |
| `agent-diva-tooling` | 2.10 / 2.11 | 自我升级 + 插件热重载调研任务 |
| `agent-diva-swarm` | 2.9 | 保持 defer,本决策补强其理由(单一人格哲学) |

---

## 4. 待办与下次复盘触发器

### 4.1 立即可启动

- [ ] **channel 可靠性 backlog**(对应 2.5)—— 拍优先级
- [ ] **自主活动架构设计启动**(对应 2.6)—— 起草案,先定 laputa/Mentle 边界
- [ ] **自我升级 + 插件热重载 调研报告**(对应 2.10 / 2.11)—— 看 hermes + alife developer module

### 4.2 工作台 PRD 化触发条件

满足任一即触发:
- 自我升级调研完成(2.10)
- 插件系统调研完成(2.11)
- 工作台所需子能力(视觉 2.2 / 浏览 2.7)至少一项有可写规范

### 4.3 本决策的下次复盘节点

- olv2 完工 / 发布 → 重新评估 2.1(Live2D)
- 自主活动架构落稿 → 评估是否把 2.6 升级为本决策 v2
- 工作台 PRD 起稿 → 评估是否升级为本决策 v2(把工作台内子能力显式化)

---

## 5. 用户原话(决策溯源)

> 1、live2d不做,因为有版权风险,或者推迟到olv2完成
> 2、深度视觉:目前来说diva有视觉能力,视觉功能留到工作台实现
> 3、语音对话有了
> 4、长期记忆是diva下一个大版本更新的重要内容,这点已完成和超越
> 5、平台通讯待优化channel,defer.
> 6、自主活动是下一个生命周期的巨大功能,而且就是基于已完成的laputa和mentle,但是待架构设计
> 7、网上冲浪(agent浏览器)放在工作台系统中
> 8、脚本执行已做
> 9、多开不做,这个是已经defer的蜂群实现,但是和目前diva单一人格哲学冲突。
> 10、自我升级很重要,需要调研,这个也可以参考hermes
> 11、插件系统也很重要,也需要调研。但是我想把它和工作台一起设计

---

## 6. 持续调研笔记(2026-06-18 新增)

> **状态**: 提醒下次会话继续阅读历史文档以完善设计。**未完成,需要持续推进**。

### 6.1 触发原因

W2-W6 设计阶段(对应 `DECISION-v2-next-phase.md` Harness Engineering)发现:

- **历史决策散落在 `agent-diva-agent-new/docs/logs/2026-05-*/`** 多个 log 目录
- 这些 log 包含已被讨论但**未落地的设计**(GenericAgent L0-L4 分层压缩、Phase 1 公理、4-axiom 系统)
- 直接重新设计会**重复造轮子 + 违背既有决策**

### 6.2 必须继续看的旧文档(优先级降序)

| # | 文档 | 路径 | 必读原因 |
|---|------|------|---------|
| 1 | **memory-architecture-deep-dive** | `agent-diva-agent-new/docs/logs/2026-05-memory-architecture-deep-dive/v0.0.1-architecture-analysis/summary.md`(已读 1/2) | GenericAgent L0-L4 设计 + Phase 1 决策"不碰 mentle" + 4 公理 + 分类决策树 |
| 2 | **mentle-laputa-memory-role** | `agent-diva-agent-new/docs/logs/2026-05-mentle-laputa-memory-role/v0.0.1-role-decision/summary.md` | Mentle 定位("optional external semantic notebook")+ Laputa/Mentle 边界 |
| 3 | **compression-taxonomy** | `agent-diva-agent-new/docs/logs/2026-05-compression-taxonomy/v0.0.1-context-vs-rhythm-compression/summary.md` | 三种压缩区分:context compaction vs memory consolidation vs rhythm distillation |
| 4 | **laputa-architecture-audit** | `agent-diva-agent-new/docs/logs/2026-05-laputa-architecture-audit/v0.0.1-laputa-integration-feasibility/` | Laputa 集成可行性,state machine 太复杂被推迟 |
| 5 | **laputa-new-architecture** | `agent-diva-agent-new/docs/logs/2026-05-laputa-new-architecture/v0.0.1-new-laputa-design/` | Laputa 新设计草案 |
| 6 | **autodream-compression-research** | `agent-diva-agent-new/docs/logs/2026-05-autodream-compression-research/v0.0.1-compression-design/` | AutoDream 压缩设计细节 |
| 7 | **context-compaction-research** | `agent-diva-agent-new/docs/logs/2026-05-context-compaction-research/` | context-window 压缩算法选择 |
| 8 | **compression-taxonomy vs evolution** | `agent-diva-agent-new/docs/logs/2026-05-context-vs-evolution-decision/` | 压缩 vs 进化 边界 |
| 9 | **shared-memory-rendering** | `agent-diva-agent-new/docs/logs/2026-05-shared-memory-rendering/` | 共享 memory 渲染 |
| 10 | **mentle-runtime** | `agent-diva-agent-new/docs/logs/2026-05-mentle-runtime/` | Mentle runtime 集成细节 |

### 6.3 已知但还没读完的

- `agent-diva-agent-new/docs/logs/2026-05-memory-architecture-deep-dive/v0.0.1-architecture-analysis/` 还差 `acceptance.md` / `verification.md` / `release.md` 没看
- `agent-diva-pro/docs/dev/archive(old-docs-dont-read-me)/morediva-root/` 整个目录是 archive(可能不读),但需确认是否有非 archive 内容

### 6.4 调研优先级建议(下次会话起手)

1. **先读未读完整文档**(上面 #4 #5 #6 #7 #8 #9 #10 的完整版本)
2. **再扫 `agent-diva-pro/docs/dev/genericagent/`** 目录(可能有 4 公理 + 决策树的源文件)
3. **检查 `agent-diva-pro/docs/dev/archive(old-docs-dont-read-me)/`** 是否真的可忽略
4. **如发现矛盾设计,优先遵循 `2026-05-*` 的决策**,而不是自己重判

### 6.5 关键约束(调研时必须遵守)

> **不要违反 Phase 1 决策**: memtle 不进核心循环,laputa 状态机推到 Phase 2,纯文件 + 公理是基础。

---

## 7. 相关决策引用

- **DECISION-v2-next-phase.md** —— Harness Engineering 6 cats(W2 Tools & permissions 是 cronjob 整合点)
- **docs/research/alife-harness-gap-inventory.md** —— alife vs diva 14 项 gap(已通过这文件调研发现)
- **docs/design-notes/autonomous-activity-thoughts.md** —— 4 层架构(延后到 v2+,不影响本决策)
- **docs/research/harness-engineering-three-way-detailed-checklist.md** —— 22 维度三方对比
