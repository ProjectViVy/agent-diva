# SOP / Skill Product Model（GenericAgent）

- 基线：`ee5a474`
- **禁止在此文档发明晋升状态机**

## 1. 术语表

| 术语 | 运行时含义 | 营销含义 |
| --- | --- | --- |
| **Skill** | L3 SOP/脚本的非正式同义词 | “结晶执行路径 / 技能树 / 市场包” |
| **SOP** | `memory/` 下 playbook（常 `*_sop.md`） | 战斗验证流程 |
| **L0 META-SOP** | `memory_management_sop.md` | Meta Rules |
| **L1 Insight** | `global_mem_insight.txt` | 路由索引 |
| **L3** | 任务 Skills/SOPs 文件树 | 能力库 |
| **Skill Marketplace / Sophub** | 无 in-repo loader | 外链 / coming soon |
| **skills/** | **本树不存在** | 安装文档保留用户本地路径 |
| **Working / related_sop** | 会话绑定当前 SOP 指针 | 非 Skill 资产 |

## 2. 运行时定义

> **Skill（实践）= 可被发现的 L3 文件集合**：  
> 用 L1 发现 → `file_read` 加载 → 可选 `code_run` 执行 helper → 成功后 `start_long_term_update` 最小 patch 精炼。

**不存在：** Skill 类、skill ID 注册表、enable 标志、semver、package manifest、`load_skill(name)` API。

## 3. 产物形状

| 形状 | 例 | 用途 |
| --- | --- | --- |
| `*_sop.md` | `plan_sop.md`, `web_setup_sop.md` | 主 playbook |
| 非 `_sop` md | `subagent.md`, `computer_use.md` | 同机制 |
| helper `.py` | `ljqCtrl.py`, `checklist_helper.py` | 可执行面 |
| SOP 目录 | `autonomous_operation_sop/`, `review_sop/` | 包式文档 |
| 模板 | `vision_api.template.py` | 物化后使用 |

本地约 **22** 个 `*_sop.md`（另有 helper 与非后缀 md）。

### 分类盘点

| 类 | 代表 |
| --- | --- |
| 元记忆 | `memory_management_sop`, `memory_cleanup_sop` |
| 编排/多 agent | `plan_sop`, `subagent`, `verify_sop`, `goal_*`, `checklist_*`, `ultraplan`, `supervisor` |
| 自主/定时 | `autonomous_operation_sop`, `scheduled_task_sop` |
| 桌面/视觉/移动 | `computer_use`, `ljqCtrl*`, `vision_sop`, `adb_ui`, `ui_detect` |
| 浏览器 | `web_setup_sop`, `tmwebdriver_sop`, `vue3_component_sop` |
| 吸收/复制 | `morphling_sop`, `incubator_sop`, `project_mode_sop` |
| 开发协作 | `github_contribution_sop`, `code_review_principles`, `review_sop` |

## 4. 生命周期（实际）

| 阶段 | 机制 | 一等 API？ |
| --- | --- | --- |
| Discover | L1 注入 + `ls memory/` | 否 |
| Load | `file_read` | 否（通用文件） |
| Bind | `update_working_checkpoint.related_sop` | 会话级 |
| Update | `file_patch` / 少用 `file_write` | 否 |
| Crystallize | `start_long_term_update` + L0 | 触发器，非 writer |
| Delete | 约定删 L1 映射；无 `file_delete` 工具 | 否 |
| Conflict | patch 唯一匹配失败；无 merge 引擎 | 软 |
| Visibility | L1 常驻软可见；L3 按需 | 否启停 |

**无：** draft→candidate→promoted 层级。结晶 = **直接写文件**。

## 5. 版本 / 启停 / 测试 / 外部修改

| 能力 | 现状 |
| --- | --- |
| Semver | 缺失 |
| Enable/disable | 缺失（≈ 不引用 / 删指针） |
| 形式化测试 | 弱：`subagent.md` 可测 SOP 质量；无 CI skill suite |
| 外部编辑再发现 | 隐式：下次 file_read / 新会话见磁盘；无 watcher |
| 访问统计 | `file_access_stats.json`（telemetry，非排序 API） |

## 6. 营销 vs 运行时

| 宣称 | 现实 |
| --- | --- |
| Don't preload skills | 预装大量 SOP |
| Automatically crystallize | 模型可选调用；非编译器 |
| Million-scale Skill Library | 外站新闻；非本 repo 格式 |
| You don't need to manage Skills | 无 UI；管理 = 文件 + L1 纪律 |

## 7. 与 Diva Skill runtime 的关系（事实）

| | GA | Diva |
| --- | --- | --- |
| 载体 | `memory/*` 松散文件 | `skills/<name>/SKILL.md` |
| 加载器 | 无专用；file_read | `SkillsLoader` + context section |
| 进化闭环 | L0 引导写 memory | **与 Evolution 脱钩**；AutoDream 不写 skills/ |
| 管理 UI | 无 | Settings Skills；Evolution 管提案 |

## 8. 稳定设计 vs 偶然实现

| 稳定（多提交巩固） | 偶然/软 |
| --- | --- |
| L0 公理 + L1–L3 文件树 | “Skill” 一词 |
| start_long_term_update 触发器模式 | 15+ turns 必须 |
| file_patch 作为突变原语 | marketplace 包 |
| L1 存在性编码 | 具体 SOP 文件名集合 |
| `--no-user-tools` 禁结算 | 文件命名 `_sop` 后缀（非强制） |
