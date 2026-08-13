# Diva Evolution × GenericAgent 差距分析

- 状态：`Research Draft`
- 日期：2026-08-13
- 配套：`r0-evolution-slice.md`、GA 基线文档
- GenericAgent = **设计参考 only**

## 1. 对比矩阵

| 维度 | GenericAgent | Diva Evolution 今 | 差距 |
| --- | --- | --- | --- |
| 触发 | 任务内 `start_long_term_update`（软）+ reflect | 后台 AutoDream run | 任务内 vs 批处理 |
| 验证 | L0 公理（软）+ plan VERIFY | evidence_refs + CandidateGate（治理形状） | 有形状、缺行动验证可证明性 |
| 产物 | L3 SOP + scripts（称 Skill） | Memory/Persona proposal patch；几乎不写 `skills/` | 对象错位 |
| 权威 | 文件即权威（高信任） | proposal + receipt 后写 | Memory 决策要求退出审批 |
| 可管理性 | 文件浏览/编辑 | 提案 inbox/runs；Skill 在设置页 | 管理对象错误 |
| 审批 | 记忆靠自律 | governance ledger 过宽 | 与 D1 冲突 |
| Skill 加载 | file_read SOP | SkillsLoader | 加载器可借鉴；闭环不可复用 |
| Persona | 不进独立人格工作区 | Identity* 提案 | Evolution 不得承载 |

## 2. 三分类

### 2.1 可采纳

| 项 | 理由 |
| --- | --- |
| Action-Verified 公理精神 | Skill 入权威的**内容门槛**，非通用 approval |
| 任务内/任务后结晶触发 | 贴近“成功任务形成能力” |
| 最小充分指针 / 极简索引 | Skill 发现索引 |
| 经验写成可执行载体 | 与 `SKILL.md` 方向一致 |
| 禁易变/未验证猜测 | 写入过滤规则 |
| Subagent 禁 LTM | 后台批处理不污染权威 |

### 2.2 不适合

| 项 | 理由 |
| --- | --- |
| Agent 任意 file_patch 长期权威 | 破坏桌面安全与域隔离 |
| L0–L4 文件布局整搬 | BML + Persona Markdown 已冻结 |
| Skill = 任意 `memory/*_sop.md` 命名 | 非严格领域模型 |
| Memory 与 Skill 同一蒸馏工具 | D1/D3 |
| 扩展旧 ProposalType/Governance | D2 |

### 2.3 必须重设计

| 项 | 说明 |
| --- | --- |
| Evolution 单一业务对象 | 从「提案+Run」→「可管理 Skill」 |
| 触发×验证×产物闭环 | 候选如何证明可复用并进入 `skills/` |
| 审批边界 | Skill 内容审查 ≠ 危险工具 ≠ Persona Diff |
| SOP↔Skill 关系 | 研究未完禁止晋升机；也禁止假装已有 |
| 权威隔离 | skills 树 / BML / Persona 物理类型隔离 |
| GUI IA | 查看/搜索/启停/编辑/验证/删除/来源/版本 |
| 旧链删除 | D4 保护分支后非兼容删除 |

## 3. 用户可管理性任务对照

| 任务 | GA | Diva Evolution 今 |
| --- | --- | --- |
| 查看能力 | ls memory / L1 | Settings Skills；Evolution 无 |
| 搜索 | 文件关键词 | 提案过滤 |
| 启停 | 不引用 | always 元数据；产品化弱 |
| 编辑 | file_patch | proposed_patch / 上传 |
| 验证 | 再跑任务 | 无“重测 Skill” |
| 删除/恢复 | 删文件 | reject/rollback；Skill delete 独立 |
| 来源 | 弱（git） | 强（run_id）但追踪提案 |
| 冲突 | 人工 | suppression / stale |

**结论：** Diva 治理可观测更强，能力资产可管理更弱。

## 4. 研究级删除候选（非实施）

| 候选 | 标记 |
| --- | --- |
| autodream 作为 Evolution 主链 | DEL/DECIDE（报告是否独立） |
| evolution 混合 Proposal/AutoDream DTO | SPLIT |
| laputa proposals 作 Evolution 入口 | DEL（对 Evolution） |
| MemoryGovernance coordinator | DEL（对 Memory/Evolution） |
| EvolutionView 四页签 | DEL（IA 重做） |
| SkillsLoader / skills/ | **KEEP** |
| BML | **KEEP** |
| Chat Approval Center | **KEEP** |
| `/api/autodream/**` | DEL |
| `.agent-diva/autodream/**` | DEL |
| `.laputa/proposals/**`（Evolution 用） | DEL |
| Memory 向 governance.sqlite3 映射 | DEL |
| `autodream_laputa_e2e` 等 | DEL 或改删除证明 |

负向证明应断言：无 AutoDream 产品入口；无 Evolution inbox 依赖 Memory proposal；Skill 变更不写 governance；无 personality/memory/skill 捆绑审批文案。

## 5. 历史 inventory 处置

`docs/logs/2026-08-05-memory-ga-parity-inventory/`：

- **保留：** GA L0–L4 描述、代码地图线索、公理摘要  
- **作废作为目标：** “对齐 G 治理闭环即成功”、“记忆写权威必须 HITL”（Memory 已被 D1 修正）

## 6. 结论

1. 现状是**治理化 AutoDream 提案工作台**，不是 Skill 自进化工作台。  
2. 可采纳纪律与结晶哲学；不可采纳任意写权威与 memory 一体布局。  
3. 必须重设计对象/权威/闭环/审批/GUI；禁止小修旧 inbox。  
4. 删除面跨 autodream、evolution DTO、laputa proposal/governance、manager、GUI、e2e。  
5. **完整 R0 未完成**；本 gap 不足以单独通过全量 Research Gate，但满足 R1 Evolution 研究输入。
