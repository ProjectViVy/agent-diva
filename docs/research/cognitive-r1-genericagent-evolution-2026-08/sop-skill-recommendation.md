# SOP / Skill 关系建议（研究级）

- 状态：`Research Recommendation — Not Architecture`
- 日期：2026-08-13
- 约束：**证据不足时不发明晋升层级或状态机**

## 1. 证据摘要

| 来源 | 关于 SOP/Skill 的事实 |
| --- | --- |
| GA README | Skill = 结晶执行路径；L3 = Task Skills/SOPs |
| GA CONTRIBUTING | Skill 落地 = `memory/` 文件 |
| GA 运行时 | 无 Skill 类型；无 `skills/`；SOP 文件 + helper |
| GA 结晶 | `start_long_term_update` 软触发 → file_patch |
| Diva Skill runtime | `skills/*/SKILL.md` + SkillsLoader |
| Diva Evolution | 管提案/Run；`SopCreate` 僵尸类型被 gate 拒绝 |
| 产品决策 D3 | Evolution 只管理 Skill；SOP 关系由研究定 |

## 2. 候选关系模型（仅假设，不冻结）

### 假设 A — 内容与载体（**当前证据最支持**）

- **SOP 内容** = 可复用程序知识（步骤、坑、前置条件）  
- **Skill 载体** = 在 Diva 中可被 loader 发现与注入的包（`SKILL.md` ± 资源）  
- GA 中二者同一文件系统角色；Diva 应用 **一个用户可见对象（Skill）**，正文可含 SOP 式 playbook  

**支持证据：** GA 无两阶段 API；结晶直接写文件；Diva 已有 Skill 加载器。  
**不支持：** 需要独立 “SOP 资产库” 的证据不足。

### 假设 B — 两阶段（自动 SOP → 用户固化 Skill）

- 自动沉淀草稿 playbook，用户选择发布为 Skill  

**状态：** decision-record 明确为**待验证假设**，**禁止现在实现**。  
**缺口：** GA 无草稿阶段；活体实验未测模型草稿质量；无用户任务数据证明两阶段必要。

### 假设 C — 两个独立对象

- SOP 与 Skill 分库分生命周期  

**风险：** 重蹈 “多权威 / 提案类型膨胀”；与 clean-break 简化目标冲突。  
**证据：** 不足；不推荐作为默认。

## 3. 研究建议（给 D0/D3，非批准）

1. **默认采用假设 A 的叙事准备**：用户只管理 **Skill**；Skill 文档形态可吸收 SOP 写作纪律（极简、坑点、前置、Action-Verified）。  
2. **不要**在 Research Gate 前实现 Candidate→Published 状态机。  
3. 若未来实验证明需要草稿，再单独立项验证假设 B；默认路径应能在无草稿机下工作。  
4. L2 环境事实 **不属于** Evolution；属 Memory/BML。  
5. L1 式索引可作为 Skill 发现层灵感，但是 Evolution 域内索引，不是 Memory insight 复用。  
6. Action-Verified 应升级为 **代码可检查门槛**（证据绑定），不能只抄 L0 文案。  
7. 触发优先研究 **任务成功路径上的结晶**；AutoDream 批处理若保留，对象不得再是 Memory/Persona patch。

## 4. 回答 decision-record 10 问（简表）

| # | 问题 | 回答 | 级别 |
| --- | --- | --- | --- |
| 1 | 更新 main 差异 | 本地 `ee5a474` vs 远程 `f06d550`；同 tree twin `9d99e9fc`；远程含 turn&lt;10 拦截等本地后提交 | 提交事实 |
| 2 | 稳定 vs 偶然 | 稳定：L0–L4、结算触发器、file_patch、L1 索引；偶然：Skill 词、15+ turns、marketplace | 源码事实+推断 |
| 3 | Action-Verified | 仅 prompt；无写门；失败/敏感靠纪律 | 源码事实 |
| 4 | 去重/合并/删/恢复 | 文件纪律 + cleanup SOP；无正式合并/版本/恢复 API | 源码事实 |
| 5 | SOP vs Skill | 证据偏向 **一个对象（Skill）+ SOP 式内容**；两阶段未验证 | 建议 |
| 6 | 可管理性任务 | 见 gap §3；今 Evolution 不满足 Skill 任务 | 源码事实 |
| 7 | 审批 | Memory/Skill 管理不应进危险工具 ledger；执行危险动作才进 Approval Center | 建议+决策 |
| 8 | 与 Persona 隔离 | 物理：skills/ vs persona md；类型：禁止 Identity 路由能力 | 建议 |
| 9 | 旧链删除 | 见 gap 删除候选；R4 细化为证明目录 | 研究级 |
| 10 | 验收场景 | 真实/重复/失败/恶意/重启；P1 活体仍缺密钥 | 建议 |

## 5. 纵向验收场景（研究设计，未执行）

| 场景 | 期望观察点 |
| --- | --- |
| 真实任务成功 | 仅验证事实进入 Skill；可再次 file_read/load 复用 |
| 重复任务 | 去重/合并不膨胀索引 |
| 失败任务 | 不写入权威；失败结论不固化 |
| 恶意/敏感 | 密钥不进索引；注入不进 Skill 正文权威 |
| 重启 | Skill 文件仍可被 loader 发现 |
| 外部编辑 Skill | 下次会话可见（或显式 reload） |
| Subagent | 默认不结晶全局 Skill |

## 6. 明确未决（禁止默认选型）

1. 是否需要草稿层（假设 B）  
2. Skill 权威是否仅文件，是否另需索引 DB  
3. Action-Verified 机器证据格式（tool hash？测试命令？）  
4. 与 marketplace 安装 Skill 的信任统一  
5. 批处理蒸馏是否保留及对象  
6. 保护分支命名与基线 commit（R4）  

## 7. 一句话建议

> 在 Diva 中把 **Skill 作为 Evolution 唯一用户对象**；用 GA 的 **L0 纪律 + 任务结晶触发 + 极简发现索引** 作为设计灵感；用 Diva 的 **`skills/` loader + 强域隔离 + 代码级证据门** 替代 GA 的任意写文件权威与 Diva 旧 AutoDream 提案链。  
> **SOP 先视为 Skill 的内容写作形态，而不是第二个产品对象**——直到有实验证明必须拆分。
