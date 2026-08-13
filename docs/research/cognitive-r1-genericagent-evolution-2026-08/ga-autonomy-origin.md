# GenericAgent 自主进化从哪来

- 状态：`R1c Research / Awaiting User Read`
- 日期：2026-08-14
- 基线：本地 `.workspace/genericagent` @ `ee5a474`（与 R1 相同）
- 性质：因果说明。**不设计 Diva Evolution，不授权 D3。**
- 活体实验：本页不做。若以后跑，用桌面 `keys.txt`。

## 一句话

GA 没有独立「进化引擎」。**能力变长 = 模型自己用文件工具改 `memory/`。**  
所谓自主，只是定时器再丢一条假用户任务，让同一套循环再跑一遍。  
**没有任何代码门**保证「做完任务就结晶 Skill」。

---

## 1. README 说的，和代码做的

README（Self-Evolution Mechanism）：新任务 → 探索 → **自动**结晶成 Skill → 下次直接调用。

源码事实：仓库里 **没有** `skills/` 注册表、没有晋升状态机、没有任务结束钩子去写 Skill。  
运行时的「Skill」= `memory/` 下的 `*_sop.md` 和可选 `.py`。下次能「一句话调用」，是因为 L1 索引或 SOP 里写了怎么调，且模型愿意 `file_read`。

| 叙事 | 运行时 |
| --- | --- |
| 自动结晶 Skill | 模型自愿 `file_patch` / `file_write` |
| 专属技能树 | 就是 `memory/` 目录越积越多 |
| 自我进化框架 | 同一 Agent Loop + 记忆文件 + 可选定时器 |

---

## 2. 外环：所谓 Autonomous 从哪来

必须显式 `--reflect <脚本>` 才有外环。普通 `>` 交互循环 **不会**自己进化。

| 脚本 | 间隔 | `check()` 实际做什么 |
| --- | --- | --- |
| `reflect/autonomous.py` | 1800s | 返回固定中文：「用户已经离开超过 30 分钟…」**没有离开检测** |
| `reflect/scheduler.py` | 120s | 扫 `sche_tasks/` 到期任务；另每 12h 跑 L4 压缩 |
| `reflect/goal_mode.py` | 5s | 预算循环；prompt 要求不写全局记忆 |
| checklist / team | 60s | BBS 另线 |

`agentmain.py` 反射环：`check()` 有字符串 → `put_task(..., source='reflect')` → 等 `done` → 睡 `INTERVAL`。  
这只是 **再塞一条 query**。结算长期记忆与否，仍看模型在这条任务里会不会调工具。

**源码事实：** Autonomous ≠ 后台进化守护进程。

---

## 3. 内环：能力怎么写上盘

同一套 `agent_runner_loop`。记忆写入是工具副作用。

```text
用户 或 reflect 丢来一条 query
        │
        ▼
  多 turn 工具循环（最多约 180 轮）
        │
        ├─ 模型可选 start_long_term_update
        │     只注入 L0（memory_management_sop.md）+ 蒸馏提示
        │     此函数本身不写任何记忆文件
        │
        └─ 模型再 file_read / file_patch / file_write
              才改 L1 / L2 / L3（*_sop.md、.py）
```

`ga.py` `do_start_long_term_update`：构造「请提取经验证成功的长期信息」的 `next_prompt`，`tool_result` = `"This is L0:\n" + SOP`。注释写「任务完成后有重要信息时调用」。

**源码事实：** 任务结束（无工具的自然语言 / `ask_user` / 超轮）**不会**自动调用 `start_long_term_update`。  
schema/提示说 15+ 轮必须结算，模型可忽略。

`--no-user-tools`（子代理常用）从 schema **拿掉** `start_long_term_update` 和 `ask_user`。

L4 是 scheduler 每 12h 压缩 `temp/model_responses`，不是 Skill 晋升。

---

## 4. 有没有代码门

| 说法 | 强制？ |
| --- | --- |
| Action-Verified / No Execution, No Memory | **否**。只在 L0 文本 |
| 只能 patch、L1≤30 行 | **否**。SOP 自称；`file_write` 仍能整文件覆盖 |
| 写入必须绑成功工具结果 | **否**。无 `evidence_refs` |
| 15+ 轮必须蒸馏 | **否**。提示词 |
| 任务结束自动结晶 | **否**。无钩子 |
| 定时 reflect 再开一轮 | **是**（若启动了 `--reflect`） |
| `file_patch` 块必须唯一匹配 | **是**（结构，不是语义真实） |

口号是宪法，不是 API 前置条件。模型随时可改 `global_mem*.txt` / SOP。

---

## 5. 不是什么（防 Diva 抄错）

- **不是** Diva AutoDream（没有批处理提案器）。  
- **不是** EvolutionProposal / Governance。  
- **不是** 代码强制的 Skill 生命周期。  
- **不是** 检测到用户离开后的真自主。  
- **不是** 把 MemoryPatch 当进化。

对 Diva **可借鉴**：写长期东西时用提示词纪律（成功工具才记）；发现靠短索引。  
**不要抄**：1800s 假离开、任务结束自动结算（GA 自己也没有）、把定时器当进化引擎。

---

## 6. 证据分级

| 结论 | 标签 |
| --- | --- |
| 无任务结束自动结算；`start_long_term_update` 不写盘 | 源码事实 |
| autonomous 无离开检测 | 源码事实 |
| 无 `skills/`、无晋升状态机 | 源码事实 |
| Action-Verified 无写入门 | 源码事实 |
| 「下次一句话调用」靠模型再读 SOP/脚本 | 推断（符合文件布局，未活体证明） |

未跑活体。R1 Gate 是否因本页而过，仍等用户看完这句话。

---

## 7. 论文怎么说（对照源码）

正式技术报告：

- Liang, Han, et al. **GenericAgent: A Token-Efficient Self-Evolving LLM Agent via Contextual Information Density Maximization (V1.0)**
- arXiv：**[2604.17091](https://arxiv.org/abs/2604.17091)**（2026-04-18）
- PDF：https://arxiv.org/pdf/2604.17091
- HTML：https://arxiv.org/html/2604.17091v1
- 复现仓（论文侧）：https://github.com/JinyiHan99/GA-Technical-Report
- 产品仓：https://github.com/lsdefine/GenericAgent

论文把自我进化写成四件套之一：**把已验证轨迹压成可复用 SOP 和可执行代码**（§2.1.3、§2.3.3）。关键句子：

- 任务结束后，轨迹被压成长期表示写进共享记忆（§2.2）。
- 长期巩固是 **triggered commit**：先进入校验阶段，再小步写入 L2/L3（§2.3.2）。
- L3 只通过 **explicit consolidation** 生成，在子目标成功或从错误恢复等里程碑触发（§2.3.3）。
- 质量门是 **No Execution, No Memory**（论文当设计不变量写）。
- Reflect Mode：外部脚本轮询条件，命中就当普通任务丢进同一 loop（§3.2）。Watchdog / Scheduled Task 只是脚本不同。
- Autonomous Exploration（§3.3）：默认约 **6 分钟**触发；还有 skill tree、用量计数、课程规划打分公式。论文自己承认权重自适应「尚未积累足够长期数据」。

和本地 `ee5a474` 的差（**以源码为准设计 Diva，论文当叙事**）：

| 论文 | `ee5a474` 源码 |
| --- | --- |
| 任务后压缩进长期记忆 | **无**任务结束自动结算 |
| 记忆系统触发巩固 | `start_long_term_update` 由**模型可选调用**，函数本身不写盘 |
| No Execution, No Memory 当不变量 | 只在 L0 文本，**无写入 API 门** |
| 默认自主探索 ~6 分钟 + skill tree | `autonomous.py` **1800s 假离开**；全仓无 `skill_tree` 实现 |
| 进化由记忆系统触发、非用户 | 外环必须 `--reflect`；内环靠模型自愿 `file_patch` |

论文复现仓 README 写 “Reflection compresses verified trajectories… triggered by the memory system, not the user.” 这是**论文口径**。本地产品仓实现更瘦：没有单独的记忆系统守护进程去触发巩固。

对 Diva：可借鉴的是论文原则（密度、分层按需、只巩固已验证路径、工具要少）。不要把论文 §3.3 的 skill tree / 课程公式当成当前 GA 代码里已有的东西来抄。
