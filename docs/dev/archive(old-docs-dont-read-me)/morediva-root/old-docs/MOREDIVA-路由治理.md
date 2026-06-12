# MOREDIVA Canonical 路由治理规则

> **状态**: FROZEN (2026-06-06)  
> **来源**: `MOREDIVA-分支归并决策.md` + `MOREDIVA-分支归并待办卡.md` GOV-01  
> **适用范围**: `agent-diva` 全生态系统  
> **约束**: 本文件是后续所有迁移、归并、新功能落地的唯一路由准绳

---

## 1. 角色定义（Canonical）

| 仓库 | 角色 | Landing Zone | 状态 |
|------|------|-------------|------|
| `agent-diva` | 稳定后端主线 | backend / runtime / safety / infrastructure | **ACTIVE** |
| `agent-diva-pro` | 产品实现线 | product / GUI / workflow / user-visible capability | **ACTIVE** |
| `agent-diva-selfinprove` | 研究母线 | 架构研究 / 设计规格 / 决策文档 / 未来 spec | **RESEARCH-ONLY** |
| `agent-diva-agent-new` | 重叠孵化副本 | 与 selfinprove 高重叠，先去重再归档 | **PENDING-DEDUP** |
| `agent-diva-sandbox` | 历史实验线 | 安全/sandbox 资产来源，已吸收能力不再回流 | **ARCHIVED** |
| `agent-diva-channel-adapter-and-plugins` | 历史文档壳 | 仅归档/零星提取 | **ARCHIVED** |
| `agent-diva-tui` | 历史文档壳 | 仅归档/零星提取 | **ARCHIVED** |
| `agent-diva-vrm-memory-test` | 可选专项线 | VRM/memory 定向取材，非当前主归并目标 | **OPTIONAL** |

---

## 2. 路由规则（硬约束）

### 2.1 `agent-diva` (main) — 只收后端/基础设施

**允许进入的内容：**
- bugfix、安全与稳定性修复
- provider / manager / core / tooling 增强
- MemoryProvider / MemoryManager 边界内的基础能力
- 无 GUI 依赖的后端协议与数据结构
- Shared MEMORY rendering MVP
- Candidate / Evidence / Audit 最小协议层
- AutoDream trigger / lock / checkpoint / worker backend skeleton
- Context compaction 底层 runtime / budget / safety 能力

**明确禁止：**
- 重产品形态的 Journal / Inbox / Review UI
- 实验性自进化工作流全量 UI
- 大量 still-draft 设计文档全集
- 任何需要明确产品交互与导航的信息架构

### 2.2 `agent-diva-pro` (pro) — 只收产品/GUI/交互

**允许进入的内容：**
- GUI / UX / 交互功能
- 节律系统的用户可见工作流
- Journal / Inbox / ReviewCard / EvolutionProposalCard / EvidencePeekCard
- Self-evolution settings
- Thinking / multimodal / pet / advanced workflow 等产品增强
- Rhythm run status / health visibility
- SOP / Skill candidate 的产品化承载面

**明确禁止：**
- 无 GUI 依赖的后端协议层
- 底层 runtime / safety / infrastructure 改动
- 纯研究文档或架构探索

### 2.3 `agent-diva-selfinprove` (研究线) — 禁止承接新功能

**允许保留的内容：**
- 长篇调研原文
- 比较稿 / 决策稿 / 迭代日志
- 未来阶段性 spec
- 未进入实施排期的架构探索

**明确禁止：**
- ❌ 继续承接新的产品功能实现
- ❌ 继续承接新的后端代码开发
- ❌ 作为长期并行实现主线
- ❌ 大范围直接 merge 进入 main 或 pro
- ❌ 任何新的 `.rs` / `.vue` / `.ts` 代码文件

**唯一输出方式：** 按主题拆 Epic 后迁移 / 文件级 cherry-pick / 文档结论重新实现

### 2.4 历史线 (sandbox / channel-adapter-and-plugins / tui) — 冻结归档

**状态：** ARCHIVED — 不再承载任何新功能

**允许的操作：**
- 提取残值到 main 或 pro（需显式卡）
- 归档标记与文档整理

**明确禁止：**
- ❌ 任何新的功能开发
- ❌ 任何新的代码提交（除归档标记）
- ❌ 作为任何 landing zone 的来源

---

## 3. 迁移规则

### 3.1 迁移方式（允许 vs 禁止）

| 方式 | 状态 |
|------|------|
| 按主题拆 Epic 后迁移 | ✅ 允许 |
| 文件级 cherry-pick | ✅ 允许 |
| 文档结论重新实现 | ✅ 允许 |
| 先主线后产品面的分层落地 | ✅ 允许 |
| 整支 merge selfinprove → pro | ❌ 禁止 |
| 整支 merge selfinprove → main | ❌ 禁止 |
| 研究日志/历史文档/产品实现混成一条主线 | ❌ 禁止 |

### 3.2 Landing Zone 显式引用要求

**所有后续迁移卡必须：**
1. 明确标注目标 landing zone（main / pro / docs）
2. 引用本文件作为路由依据
3. 禁止模糊归属（如"先放这边再说"）

### 3.3 分流规则速查

```
功能归 `pro`       → product / GUI / workflow / user-visible
基础设施归 `main`   → backend / runtime / safety / infrastructure
研究留 `selfinprove` → 架构研究 / 设计规格 / 决策文档
历史归档进 `archive` → sandbox / channel-adapter-and-plugins / tui
```

---

## 4. 治理检查清单

每次新增功能或迁移前，必须确认：

- [ ] 目标 landing zone 是否明确？
- [ ] 内容是否符合目标分支的角色定义？
- [ ] 是否违反研究线/历史线的禁止规则？
- [ ] 迁移方式是否在允许列表中？
- [ ] 迁移卡是否显式引用本文件？

---

## 5. 例外处理

如需例外（如紧急 hotfix 跨分支），必须：
1. 在迁移卡中明确说明例外原因
2. 标注为 `EXCEPTION: <reason>`
3. 经人工审批后执行

---

## 6. 版本记录

| 日期 | 变更 | 原因 |
|------|------|------|
| 2026-06-06 | 初始冻结 | GOV-01 任务完成，建立 canonical 路由规则 |

---

**本文档是 morediva 生态系统的路由治理准绳。任何与本文件冲突的路由决策，以本文件为准。**
