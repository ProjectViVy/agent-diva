# COGNITIVE-R1：GenericAgent Evolution 模型研究

- 状态：`Research Complete (package) / Research Gate Pending User Review`
- 记录日期：2026-08-13
- EPIC：`LAPUTA-COGNITIVE-WORKSPACE-RESET` → **R1**
- 性质：研究事实与建议；**不授权**目标架构定稿或代码实施
- 依赖：全量 R0 见 [`../cognitive-r0-current-state-2026-08/README.md`](../cognitive-r0-current-state-2026-08/README.md)；本包仍含 **R0 Evolution 切片**

## 一句话结论

GenericAgent 的“自进化 / Skill”是 **提示词纪律 + 文件系统 L0–L4** 上的能力结晶：任务内调用 `start_long_term_update` 注入 L0，再由模型用 `file_patch` 写 L2/L3 并同步 L1。  
**Action-Verified 没有代码门禁**；README 的 “Skill” 在运行时 ≈ `memory/*_sop.md` + 可选 `.py`，**没有** `skills/` 包注册表或晋升状态机。

Diva 当前 Evolution 是 **AutoDream → Laputa Proposal → Governance** 的治理工作台，对象是 Memory/Persona patch，不是 Skill 生命周期；与 2026-08-13 产品决策（Evolution 只管 Skill）错位，必须重设计而非修补。

## 可复现基线

| 锚点 | 值 |
| --- | --- |
| 本地参考树 | `C:\Users\Administrator\Desktop\morediva\.workspace\genericagent` |
| 本地 HEAD | `ee5a474e5a1b6d203438d7d1dfa21da912268bb8`（2026-07-10） |
| 远程 origin | `https://github.com/lsdefine/GenericAgent` |
| 远程 main tip（2026-08-13 API） | `f06d5503808ba9d164fb583e4c500d5ce01efd4c` |
| decision-record 历史远端 | `63f9db74…`（仍为 tip 祖先） |
| 内容等价远程提交 | `9d99e9fc…`（与本地同 tree） |
| `skills/` 目录 | **不存在**（本地与远程 main） |
| 活体 LLM 实验 | **阻断**（无 `mykey.py`） |

## 阅读顺序

1. [genericagent-upstream-baseline.md](./genericagent-upstream-baseline.md) — 上游可复现与演进史  
2. [evolution-trigger-and-lifecycle.md](./evolution-trigger-and-lifecycle.md) — 触发与生命周期  
3. [action-verified-evidence-model.md](./action-verified-evidence-model.md) — Action-Verified  
4. [l0-l4-memory-architecture.md](./l0-l4-memory-architecture.md) — L0–L4  
5. [sop-skill-product-model.md](./sop-skill-product-model.md) — SOP/Skill 产物  
6. [evolution-behavior-experiments.md](./evolution-behavior-experiments.md) — 静态实验  
7. [r0-evolution-slice.md](./r0-evolution-slice.md) — Diva Evolution 现状切片  
8. [diva-evolution-gap.md](./diva-evolution-gap.md) — 差距与三分类  
9. [sop-skill-recommendation.md](./sop-skill-recommendation.md) — 关系建议（不发明状态机）

## EPIC 完成物映射

| EPIC 要求 | 本包文件 |
| --- | --- |
| `genericagent-upstream-baseline.md` | 同名 |
| `evolution-behavior-experiments.md` | 同名（+ trigger/lifecycle、action-verified、L0–L4 详文） |
| `diva-evolution-gap.md` | 同名 |
| `sop-skill-recommendation.md` | 同名 |

## 证据分级

| 标签 | 含义 |
| --- | --- |
| 源码事实 | 当前本地树或可定位实现 |
| 提交事实 | git / GitHub API |
| 实验观察 | 本包静态实验；活体未跑 |
| 推断 | 由事实推导 |
| 建议 | 研究建议，非架构批准 |

## 产品约束（研究不得推翻）

- Evolution 只管理 Skill 演进；Memory CRUD 不审批；Persona ≠ Memory  
- 旧 AutoDream–Evolution–Governance 链路退役，不修补  
- 研究完成前不实现 SOP Candidate → Skill 晋升状态机  
- GenericAgent 是设计参考，不是运行时依赖  

## 研究 Gate 自检

- [x] 四 EPIC 完成物齐全  
- [x] 上游版本可复现（本地锁定 + 远程 tip 记录；本地 fetch SSL 曾失败，API 补证）  
- [x] Action-Verified：代码强制 vs 提示词边界写清  
- [x] L0–L4 与 SOP/Skill 产物模型写清  
- [x] SOP/Skill 建议未发明晋升状态机  
- [x] Diva gap 三分类有证据  
- [x] R0 依赖透明（切片 + 全量包 [`../cognitive-r0-current-state-2026-08/`](../cognitive-r0-current-state-2026-08/README.md)）  
- [ ] 用户 Research Gate 评审（待）  

## 明确不做

目标 DTO/schema、GUI 布局、保护性分支创建、旧链删除、生产代码修改。
